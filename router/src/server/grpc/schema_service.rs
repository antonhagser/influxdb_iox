use crate::{
    namespace_cache::NamespaceCache,
    schema_validator::{SchemaError, SchemaValidator},
};
use data_types::{partition_template::TablePartitionTemplateOverride, ColumnType, NamespaceName};
use generated_types::influxdata::iox::schema::v1::*;
use observability_deps::tracing::warn;
use service_grpc_schema::schema_to_proto;
use std::{collections::BTreeMap, sync::Arc};
use tonic::{Request, Response, Status};

/// Implementation of the gRPC schema service that is allowed to modify schemas
#[derive(Debug)]
pub(crate) struct SchemaService<C> {
    /// Schema validator.
    schema_validator: Arc<SchemaValidator<C>>,
}

impl<C> SchemaService<C> {
    pub(crate) fn new(schema_validator: Arc<SchemaValidator<C>>) -> Self {
        Self { schema_validator }
    }
}

#[tonic::async_trait]
impl<C> schema_service_server::SchemaService for SchemaService<C>
where
    C: NamespaceCache<ReadError = iox_catalog::interface::Error> + 'static,
{
    async fn get_schema(
        &self,
        request: Request<GetSchemaRequest>,
    ) -> Result<Response<GetSchemaResponse>, Status> {
        let req = request.into_inner();

        let namespace_name = NamespaceName::try_from(req.namespace.clone())
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let schema = self
            .schema_validator
            .get_schema(&namespace_name, req.table.as_deref())
            .await
            .map_err(|e| match e {
                iox_catalog::interface::Error::NamespaceNotFoundByName { .. } => {
                    warn!(error=%e, %req.namespace, "failed to find namespace schema");
                    Status::not_found(e.to_string())
                }
                iox_catalog::interface::Error::TableNotFoundByName { .. } => {
                    warn!(error=%e, %req.namespace, ?req.table, "failed to find table schema");
                    Status::not_found(e.to_string())
                }
                _ => {
                    warn!(error=%e, %req.namespace, "failed to retrieve namespace schema");
                    Status::internal(e.to_string())
                }
            })?;

        Ok(Response::new(GetSchemaResponse {
            schema: Some(schema_to_proto(&schema)),
        }))
    }

    async fn upsert_schema(
        &self,
        request: Request<UpsertSchemaRequest>,
    ) -> Result<Response<UpsertSchemaResponse>, Status> {
        let req = request.into_inner();

        let UpsertSchemaRequest {
            namespace,
            table,
            columns,
            partition_template,
        } = req;

        let namespace_name = NamespaceName::try_from(namespace.clone())
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let namespace_schema = self
            .schema_validator
            .get_schema(&namespace_name, None)
            .await
            .map_err(|e| match e {
                iox_catalog::interface::Error::NamespaceNotFoundByName { .. } => {
                    warn!(error=%e, %namespace, "failed to find namespace schema");
                    Status::not_found(e.to_string())
                }
                _ => {
                    warn!(error=%e, %namespace, "failed to retrieve namespace schema");
                    Status::internal(e.to_string())
                }
            })?;

        let columns = convert_columns(columns)?;

        let partition_template = TablePartitionTemplateOverride::try_new(
            partition_template,
            &namespace_schema.partition_template,
        )
        .map_err(|source| {
            Status::invalid_argument(format!(
                "Could not create TablePartitionTemplateOverride for table `{table}` \
                in namespace `{namespace}`: {source}"
            ))
        })?;

        let schema = self
            .schema_validator
            .upsert_schema(
                &namespace_name,
                &namespace_schema,
                &table,
                columns,
                partition_template,
            )
            .await
            .map_err(|e| match e {
                SchemaError::ServiceLimit { .. } => Status::failed_precondition(e.to_string()),
                SchemaError::Conflict { .. } => Status::invalid_argument(e.to_string()),
                _ => Status::internal(e.to_string()),
            })?;

        Ok(Response::new(UpsertSchemaResponse {
            schema: Some(schema_to_proto(&schema)),
        }))
    }
}

fn convert_columns(columns: BTreeMap<String, i32>) -> Result<BTreeMap<String, ColumnType>, Status> {
    columns
        .into_iter()
        .map(|(name, column_type)| {
            let column_type: data_types::ColumnType =
                column_schema::ColumnType::from_i32(column_type)
                    .ok_or_else(|| {
                        Status::invalid_argument(format!(
                            "Column type {} is not a valid ColumnType",
                            column_type
                        ))
                    })?
                    .try_into()
                    .map_err(|e| {
                        Status::internal(format!(
                            "Could not convert protobuf into a ColumnType: {e}"
                        ))
                    })?;

            Ok((name, column_type))
        })
        .collect::<Result<BTreeMap<_, _>, Status>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::namespace_cache::{MemoryNamespaceCache, ReadThroughCache};
    use data_types::{ColumnType, MaxColumnsPerTable, MaxTables};
    use futures::{future::BoxFuture, FutureExt};
    use generated_types::influxdata::iox::schema::v1::schema_service_server::SchemaService;
    use iox_catalog::{
        interface::{Catalog, RepoCollection},
        mem::MemCatalog,
        test_helpers::{
            arbitrary_namespace, arbitrary_table, arbitrary_table_schema_load_or_create,
        },
    };
    use std::sync::Arc;
    use tonic::Code;

    // `SchemaService` has to be specified in this way because the `generated_types` trait is
    // also in scope. Make an alias for convenience and to have one place to explain.
    type Service = super::SchemaService<ReadThroughCache<MemoryNamespaceCache>>;

    // Given some `catalog_setup` closure that can use the catalog repos and returns a future,
    // set up the catalog, await the future, and return the gRPC service.
    async fn service_setup<S, T>(mut catalog_setup: S) -> Service
    where
        S: (FnMut(&mut dyn RepoCollection) -> BoxFuture<'_, T>) + Send,
    {
        let metrics = Default::default();
        let catalog = Arc::new(MemCatalog::new(Arc::clone(&metrics)));
        let mut repos = catalog.repositories().await;

        let setup = catalog_setup(repos.as_mut());
        setup.await;

        let schema_validator = Arc::new(SchemaValidator::new(
            Arc::clone(&catalog) as _,
            setup_test_cache(catalog),
            &metrics,
        ));

        Service::new(schema_validator)
    }

    fn setup_test_cache(catalog: Arc<dyn Catalog>) -> ReadThroughCache<MemoryNamespaceCache> {
        ReadThroughCache::new(MemoryNamespaceCache::default(), catalog)
    }

    async fn get_schema(grpc: &Service, namespace: &str, table: Option<&str>) -> NamespaceSchema {
        let request = GetSchemaRequest {
            namespace: namespace.to_string(),
            table: table.map(Into::into),
        };

        let response = grpc.get_schema(Request::new(request)).await.unwrap();
        let response = response.into_inner();
        response.schema.unwrap()
    }

    async fn get_schema_expecting_failure(
        grpc: &Service,
        namespace: &str,
        table: Option<&str>,
        expected_code: Code,
        expected_message: &str,
    ) {
        let request = GetSchemaRequest {
            namespace: namespace.to_string(),
            table: table.map(Into::into),
        };

        let status = grpc.get_schema(Request::new(request)).await.unwrap_err();
        assert_eq!(status.code(), expected_code);
        assert_eq!(status.message(), expected_message);
    }

    fn sorted_table_names(schema: &NamespaceSchema) -> Vec<String> {
        let mut table_names: Vec<_> = schema.tables.keys().cloned().collect();
        table_names.sort();
        table_names
    }

    fn sorted_column_names(schema: &NamespaceSchema, table: &str) -> Vec<String> {
        let mut column_names: Vec<_> = schema
            .tables
            .get(table)
            .unwrap()
            .columns
            .keys()
            .cloned()
            .collect();
        column_names.sort();
        column_names
    }

    #[tokio::test]
    async fn get_schema_works() {
        let namespace = "namespace_schema_test";
        let table = "schema_test_table";
        let column = "schema_test_column";
        let another_table = "another_schema_test_table";
        let another_column = "another_schema_test_column";

        let grpc = service_setup(|repos| {
            async {
                let namespace = arbitrary_namespace(&mut *repos, namespace).await;

                let table = arbitrary_table(&mut *repos, table, &namespace).await;
                repos
                    .columns()
                    .create_or_get(column, table.id, ColumnType::Tag)
                    .await
                    .unwrap();

                let another_table = arbitrary_table(&mut *repos, another_table, &namespace).await;
                repos
                    .columns()
                    .create_or_get(another_column, another_table.id, ColumnType::Tag)
                    .await
                    .unwrap();
            }
            .boxed()
        })
        .await;

        // request all tables for a namespace
        let schema = get_schema(&grpc, namespace, None).await;
        assert_eq!(sorted_table_names(&schema), [another_table, table]);
        assert_eq!(sorted_column_names(&schema, table), [column]);

        // request one table for a namespace
        let schema = get_schema(&grpc, namespace, Some(table)).await;
        assert_eq!(sorted_table_names(&schema), [table]);
        assert_eq!(sorted_column_names(&schema, table), [column]);

        // request a nonexistent table for a namespace, which fails
        get_schema_expecting_failure(
            &grpc,
            namespace,
            Some("does_not_exist"),
            Code::NotFound,
            "table does_not_exist not found",
        )
        .await;
    }

    mod upsert_schema {
        use super::*;
        use generated_types::influxdata::iox::partition_template::v1::{
            template_part, PartitionTemplate, TemplatePart,
        };
        use std::collections::BTreeMap;

        async fn upsert_schema(grpc: &Service, request: UpsertSchemaRequest) -> NamespaceSchema {
            let response = grpc.upsert_schema(Request::new(request)).await.unwrap();
            let response = response.into_inner();
            response.schema.unwrap()
        }

        async fn upsert_schema_expecting_error(
            grpc: &Service,
            request: UpsertSchemaRequest,
            expected_code: Code,
            expected_message: &str,
        ) {
            let status = grpc.upsert_schema(Request::new(request)).await.unwrap_err();
            assert_eq!(status.code(), expected_code);
            assert_eq!(status.message(), expected_message);
        }

        #[tokio::test]
        async fn nonexistent_namespace_fails() {
            let grpc = service_setup(|_repos| async {}.boxed()).await;

            // attempt to upsert into a nonexistent namespace, which fails
            let request = UpsertSchemaRequest {
                namespace: "namespace_does_not_exist".to_string(),
                table: "arbitrary".to_string(),
                columns: [("temperature".to_string(), ColumnType::I64 as i32)].into(),
                partition_template: None,
            };

            upsert_schema_expecting_error(
                &grpc,
                request,
                Code::NotFound,
                "namespace namespace_does_not_exist not found",
            )
            .await;
        }

        #[tokio::test]
        async fn new_table() {
            let namespace = "namespace_schema_upsert_new";
            let table = "table_schema_upsert_new";
            let columns: BTreeMap<_, _> =
                [("temperature".to_string(), ColumnType::I64 as i32)].into();

            let grpc = service_setup(|repos| {
                async {
                    let namespace = arbitrary_namespace(&mut *repos, namespace).await;
                    // This table should not be returned because we're not upserting it
                    arbitrary_table(&mut *repos, "existing_table", &namespace).await;
                }
                .boxed()
            })
            .await;

            let request = UpsertSchemaRequest {
                namespace: namespace.to_string(),
                table: table.to_string(),
                columns: columns.clone(),
                partition_template: None,
            };

            let schema = upsert_schema(&grpc, request).await;
            assert_eq!(sorted_table_names(&schema), [table]);
            assert_eq!(sorted_column_names(&schema, table), ["temperature", "time"]);
        }

        #[tokio::test]
        async fn new_table_with_partition_template() {
            let namespace = "namespace_schema_upsert_new_partition_template";
            let table = "table_schema_upsert_new_partition_template";
            let columns: BTreeMap<_, _> = [
                ("temperature".to_string(), ColumnType::I64 as i32),
                ("region".to_string(), ColumnType::Tag as i32),
            ]
            .into();
            let partition_template = PartitionTemplate {
                parts: vec![
                    TemplatePart {
                        // Tag *not* mentioned in the columns to insert
                        part: Some(template_part::Part::TagValue("color".into())),
                    },
                    TemplatePart {
                        // Tag *mentioned* in the columns to insert
                        part: Some(template_part::Part::TagValue("region".into())),
                    },
                    TemplatePart {
                        part: Some(template_part::Part::TimeFormat("%Y".into())),
                    },
                ],
            };

            let grpc = service_setup(|repos| {
                async {
                    arbitrary_namespace(&mut *repos, namespace).await;
                }
                .boxed()
            })
            .await;

            let request = UpsertSchemaRequest {
                namespace: namespace.to_string(),
                table: table.to_string(),
                columns: columns.clone(),
                partition_template: Some(partition_template.clone()),
            };

            let schema = upsert_schema(&grpc, request).await;
            assert_eq!(sorted_table_names(&schema), [table]);
            assert_eq!(
                schema.tables.get(table).unwrap().partition_template,
                Some(partition_template)
            );
            // The "color" tag only used in the partition template gets created too
            assert_eq!(
                sorted_column_names(&schema, table),
                ["color", "region", "temperature", "time"]
            );
        }

        #[tokio::test]
        async fn invalid_partition_template_fails() {
            let namespace = "namespace_schema_upsert_new_partition_template";
            let table = "table_schema_upsert_new_partition_template";
            let columns: BTreeMap<_, _> = [
                ("temperature".to_string(), ColumnType::I64 as i32),
                ("region".to_string(), ColumnType::Tag as i32),
            ]
            .into();
            let invalid_partition_template = PartitionTemplate { parts: vec![] };

            let grpc = service_setup(|repos| {
                async {
                    arbitrary_namespace(&mut *repos, namespace).await;
                }
                .boxed()
            })
            .await;

            let request = UpsertSchemaRequest {
                namespace: namespace.to_string(),
                table: table.to_string(),
                columns: columns.clone(),
                partition_template: Some(invalid_partition_template.clone()),
            };

            upsert_schema_expecting_error(
                &grpc,
                request,
                Code::InvalidArgument,
                "Could not create TablePartitionTemplateOverride for table \
                `table_schema_upsert_new_partition_template` in namespace \
                `namespace_schema_upsert_new_partition_template`: \
                Custom partition template must have at least one part",
            )
            .await;

            let schema = get_schema(&grpc, namespace, None).await;
            let table_names = sorted_table_names(&schema);
            assert!(
                table_names.is_empty(),
                "Expected no tables to be created; got {table_names:?}"
            );
        }

        #[tokio::test]
        async fn new_table_no_columns() {
            let namespace = "namespace_schema_upsert_no_columns";
            let table = "table_schema_upsert_no_columns";
            let columns = BTreeMap::default();

            let grpc = service_setup(|repos| {
                async {
                    arbitrary_namespace(&mut *repos, namespace).await;
                }
                .boxed()
            })
            .await;

            let request = UpsertSchemaRequest {
                namespace: namespace.to_string(),
                table: table.to_string(),
                columns: columns.clone(),
                partition_template: None,
            };

            let schema = upsert_schema(&grpc, request).await;
            assert_eq!(sorted_table_names(&schema), [table]);
            // The time column always gets created
            assert_eq!(sorted_column_names(&schema, table), ["time"]);
        }

        #[tokio::test]
        async fn existing_table() {
            let namespace = "namespace_schema_upsert_existing";
            let table = "table_schema_upsert_existing";

            let existing_columns: BTreeMap<_, _> = [
                ("name".to_string(), ColumnType::String),
                ("time".to_string(), ColumnType::Time),
            ]
            .into();

            let upsert_columns: BTreeMap<_, _> = [
                // One new column
                ("region".to_string(), ColumnType::Tag as i32),
                // One existing column
                ("name".to_string(), ColumnType::String as i32),
                // no time column, but that always gets created
            ]
            .into();

            let grpc = service_setup(|repos| {
                let existing_columns = existing_columns.clone();
                async move {
                    let namespace = arbitrary_namespace(&mut *repos, namespace).await;

                    let table = arbitrary_table(&mut *repos, table, &namespace).await;
                    for (existing_name, existing_type) in existing_columns {
                        repos
                            .columns()
                            .create_or_get(&existing_name, table.id, existing_type)
                            .await
                            .unwrap();
                    }
                }
                .boxed()
            })
            .await;
            let request = UpsertSchemaRequest {
                namespace: namespace.to_string(),
                table: table.to_string(),
                columns: upsert_columns.clone(),
                partition_template: None,
            };

            let schema = upsert_schema(&grpc, request).await;
            assert_eq!(sorted_table_names(&schema), [table]);
            assert_eq!(
                sorted_column_names(&schema, table),
                ["name", "region", "time"]
            );
        }

        #[tokio::test]
        async fn existing_table_upsert_partition_template_specified_gets_ignored() {
            let namespace = "namespace_schema_upsert_existing_partition_template";
            let table = "table_schema_upsert_existing_partition_template";

            let partition_template = PartitionTemplate {
                parts: vec![TemplatePart {
                    part: Some(template_part::Part::TimeFormat("%Y".into())),
                }],
            };

            let grpc = service_setup(|repos| {
                async {
                    let namespace = arbitrary_namespace(&mut *repos, namespace).await;
                    arbitrary_table(&mut *repos, table, &namespace).await;
                }
                .boxed()
            })
            .await;

            let request = UpsertSchemaRequest {
                namespace: namespace.to_string(),
                table: table.to_string(),
                columns: Default::default(),
                partition_template: Some(partition_template),
            };

            let schema = upsert_schema(&grpc, request).await;
            assert_eq!(sorted_table_names(&schema), [table]);
            let schema_table = schema.tables.get(table).unwrap();
            // The partition template gets ignored because the table already existed
            assert_eq!(schema_table.partition_template, None);
        }

        #[tokio::test]
        async fn conflicting_column_types_fails() {
            let namespace = "namespace_schema_upsert_conflict";
            let table = "table_schema_upsert_conflict";

            let existing_columns: BTreeMap<_, _> = [
                ("name".to_string(), ColumnType::String),
                ("time".to_string(), ColumnType::Time),
            ]
            .into();

            let upsert_columns: BTreeMap<_, _> = [
                // New column
                ("cpu".to_string(), ColumnType::Tag as i32),
                // Same name as an existing column but a different type
                ("name".to_string(), ColumnType::I64 as i32),
                // Another new column
                ("temperature".to_string(), ColumnType::F64 as i32),
            ]
            .into();

            let grpc = service_setup(|repos| {
                let existing_columns = existing_columns.clone();
                async move {
                    let namespace = arbitrary_namespace(&mut *repos, namespace).await;

                    let table = arbitrary_table(&mut *repos, table, &namespace).await;
                    for (existing_name, existing_type) in existing_columns {
                        repos
                            .columns()
                            .create_or_get(&existing_name, table.id, existing_type)
                            .await
                            .unwrap();
                    }
                }
                .boxed()
            })
            .await;

            let request = UpsertSchemaRequest {
                namespace: namespace.to_string(),
                table: table.to_string(),
                columns: upsert_columns.clone(),
                partition_template: None,
            };

            upsert_schema_expecting_error(
                &grpc,
                request,
                Code::InvalidArgument,
                "schema conflict: table table_schema_upsert_conflict, \
                column name is type string but schema update has type i64",
            )
            .await;

            let schema = get_schema(&grpc, namespace, Some(table)).await;
            assert_eq!(sorted_table_names(&schema), [table]);
            // No new columns should be added
            let mut expected_columns: Vec<_> = existing_columns.keys().cloned().collect();
            expected_columns.sort();
            assert_eq!(sorted_column_names(&schema, table), expected_columns);
        }

        #[tokio::test]
        async fn multiple_conflicting_column_types_returns_error_on_first_encountered() {
            let namespace = "namespace_schema_upsert_two_conflicts";
            let table = "table_schema_upsert_two_conflicts";

            let existing_columns: BTreeMap<_, _> = [
                ("name".to_string(), ColumnType::String),
                ("cpu".to_string(), ColumnType::String),
                ("time".to_string(), ColumnType::Time),
            ]
            .into();

            let upsert_columns: BTreeMap<_, _> = [
                // Same name as an existing column but a different type
                ("cpu".to_string(), ColumnType::Tag as i32),
                // Another column with the same name as an existing column but a different type
                ("name".to_string(), ColumnType::I64 as i32),
                // A new column
                ("temperature".to_string(), ColumnType::F64 as i32),
            ]
            .into();

            let grpc = service_setup(|repos| {
                let existing_columns = existing_columns.clone();
                async move {
                    let namespace = arbitrary_namespace(&mut *repos, namespace).await;

                    let table = arbitrary_table(&mut *repos, table, &namespace).await;
                    for (existing_name, existing_type) in existing_columns {
                        repos
                            .columns()
                            .create_or_get(&existing_name, table.id, existing_type)
                            .await
                            .unwrap();
                    }
                }
                .boxed()
            })
            .await;
            let request = UpsertSchemaRequest {
                namespace: namespace.to_string(),
                table: table.to_string(),
                columns: upsert_columns.clone(),
                partition_template: None,
            };

            upsert_schema_expecting_error(
                &grpc,
                request,
                Code::InvalidArgument,
                "schema conflict: table table_schema_upsert_two_conflicts, \
                column cpu is type string but schema update has type tag",
            )
            .await;

            let schema = get_schema(&grpc, namespace, Some(table)).await;
            assert_eq!(sorted_table_names(&schema), [table]);
            // No new columns should be added
            let mut expected_columns: Vec<_> = existing_columns.keys().cloned().collect();
            expected_columns.sort();
            assert_eq!(sorted_column_names(&schema, table), expected_columns);
        }

        #[tokio::test]
        async fn over_max_tables_fails() {
            let namespace = "namespace_schema_too_many_tables";
            let existing_table = "schema_test_table_existing";
            let upsert_table = "schema_test_table_upsert";

            let grpc = service_setup(|repos| {
                async move {
                    let namespace_in_catalog = arbitrary_namespace(&mut *repos, namespace).await;

                    // Set the max tables to 1
                    repos
                        .namespaces()
                        .update_table_limit(namespace, MaxTables::new(1))
                        .await
                        .unwrap();
                    // Create the 1 allowed table
                    arbitrary_table(&mut *repos, existing_table, &namespace_in_catalog).await;
                }
                .boxed()
            })
            .await;

            let request = UpsertSchemaRequest {
                namespace: namespace.to_string(),
                table: upsert_table.to_string(),
                columns: Default::default(),
                partition_template: None,
            };

            upsert_schema_expecting_error(
                &grpc,
                request,
                Code::FailedPrecondition,
                "service limit reached: couldn't create new table; namespace contains 1 existing \
                tables, applying this write would result in 2 tables, limit is 1",
            )
            .await;

            let schema = get_schema(&grpc, namespace, None).await;
            // Table doesn't get created
            assert_eq!(sorted_table_names(&schema), [existing_table]);
        }

        #[tokio::test]
        async fn over_max_columns_fails() {
            let namespace = "namespace_schema_too_many_columns";
            let table = "schema_test_table_existing";

            let existing_columns: BTreeMap<_, _> = [("cpu".to_string(), ColumnType::String)].into();

            let upsert_columns: BTreeMap<_, _> = [
                // One existing column
                ("cpu".to_string(), ColumnType::String as i32),
                // One new column that would put the table over the limit
                ("name".to_string(), ColumnType::I64 as i32),
            ]
            .into();

            let grpc = service_setup(|repos| {
                let existing_columns = existing_columns.clone();
                async move {
                    let namespace_in_catalog = arbitrary_namespace(&mut *repos, namespace).await;

                    // Set the max columns to 2
                    repos
                        .namespaces()
                        .update_column_limit(namespace, MaxColumnsPerTable::new(2))
                        .await
                        .unwrap();

                    let table = arbitrary_table_schema_load_or_create(
                        &mut *repos,
                        table,
                        &namespace_in_catalog,
                    )
                    .await;

                    // Create the 1 allowed column (plus the always-existing `time` column puts this
                    // table at the limit)
                    for (existing_name, existing_type) in &existing_columns {
                        repos
                            .columns()
                            .create_or_get(existing_name, table.id, *existing_type)
                            .await
                            .unwrap();
                    }
                }
                .boxed()
            })
            .await;
            let request = UpsertSchemaRequest {
                namespace: namespace.to_string(),
                table: table.to_string(),
                columns: upsert_columns.clone(),
                partition_template: None,
            };

            upsert_schema_expecting_error(
                &grpc,
                request,
                Code::FailedPrecondition,
                "service limit reached: couldn't create columns in table \
                `schema_test_table_existing`; table contains 2 existing columns, applying this \
                write would result in 3 columns, limit is 2",
            )
            .await;

            let schema = get_schema(&grpc, namespace, None).await;
            assert_eq!(sorted_table_names(&schema), [table]);
            // No new columns should be added
            assert_eq!(sorted_column_names(&schema, table), ["cpu", "time"]);
        }
    }
}
