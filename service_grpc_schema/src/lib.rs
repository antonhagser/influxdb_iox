//! Implementation of the schema gRPC service

#![deny(rustdoc::broken_intra_doc_links, rust_2018_idioms)]
#![warn(
    clippy::clone_on_ref_ptr,
    clippy::dbg_macro,
    clippy::explicit_iter_loop,
    // See https://github.com/influxdata/influxdb_iox/pull/1671
    clippy::future_not_send,
    clippy::todo,
    clippy::use_self,
    missing_debug_implementations,
    unused_crate_dependencies
)]

// Workaround for "unused crate" lint false positives.
use workspace_hack as _;

use std::{collections::BTreeMap, ops::DerefMut, sync::Arc};

use data_types::partition_template::TablePartitionTemplateOverride;
use generated_types::influxdata::iox::schema::v1::*;
use iox_catalog::interface::{
    get_schema_by_name, get_schema_by_namespace_and_table, upsert_schema_by_namespace_and_table,
    Catalog, SoftDeletedRows, UpsertSchemaError,
};
use observability_deps::tracing::warn;
use tonic::{Request, Response, Status};

/// Implementation of the gRPC schema service
#[derive(Debug)]
pub struct SchemaService {
    /// Catalog.
    catalog: Arc<dyn Catalog>,
}

impl SchemaService {
    pub fn new(catalog: Arc<dyn Catalog>) -> Self {
        Self { catalog }
    }
}

#[tonic::async_trait]
impl schema_service_server::SchemaService for SchemaService {
    async fn get_schema(
        &self,
        request: Request<GetSchemaRequest>,
    ) -> Result<Response<GetSchemaResponse>, Status> {
        let mut repos = self.catalog.repositories().await;

        let req = request.into_inner();

        let schema = match req.table {
            Some(table_name) => {
                get_schema_by_namespace_and_table(
                    &req.namespace,
                    &table_name,
                    repos.deref_mut(),
                    SoftDeletedRows::ExcludeDeleted,
                )
                .await
            }
            None => {
                get_schema_by_name(
                    &req.namespace,
                    repos.deref_mut(),
                    SoftDeletedRows::ExcludeDeleted,
                )
                .await
            }
        }
        .map_err(|e| {
            warn!(error=%e, %req.namespace, "failed to retrieve namespace schema");
            Status::not_found(e.to_string())
        })
        .map(Arc::new)?;

        Ok(Response::new(GetSchemaResponse {
            schema: Some(schema_to_proto(schema)),
        }))
    }

    async fn upsert_schema(
        &self,
        request: Request<UpsertSchemaRequest>,
    ) -> Result<Response<UpsertSchemaResponse>, Status> {
        let mut repos = self.catalog.repositories().await;
        let req = request.into_inner();

        let UpsertSchemaRequest {
            namespace,
            table,
            columns,
            partition_template,
        } = req;

        let columns = columns
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
            .collect::<Result<BTreeMap<_, _>, Status>>()?;

        let namespace_in_catalog = repos
            .namespaces()
            .get_by_name(&namespace, SoftDeletedRows::ExcludeDeleted)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found(format!("Namespace `{namespace}` not found")))?;

        let partition_template = TablePartitionTemplateOverride::try_new(
            partition_template,
            &namespace_in_catalog.partition_template,
        )
        .map_err(|source| {
            Status::invalid_argument(format!(
                "Could not create TablePartitionTemplateOverride for table `{table}` \
                in namespace `{namespace}`: {source}"
            ))
        })?;

        let schema = upsert_schema_by_namespace_and_table(
            &namespace_in_catalog,
            &table,
            columns,
            partition_template,
            repos.deref_mut(),
        )
        .await
        .map_err(|e| match e {
            UpsertSchemaError::ColumnTypeConflict { .. } => Status::invalid_argument(e.to_string()),
            UpsertSchemaError::TableLimit { .. } | UpsertSchemaError::ColumnLimit { .. } => {
                Status::failed_precondition(e.to_string())
            }
            _ => Status::internal(e.to_string()),
        })
        .map(Arc::new)?;

        Ok(Response::new(UpsertSchemaResponse {
            schema: Some(schema_to_proto(schema)),
        }))
    }
}

fn schema_to_proto(schema: Arc<data_types::NamespaceSchema>) -> NamespaceSchema {
    NamespaceSchema {
        id: schema.id.get(),
        partition_template: schema.partition_template.as_proto().cloned(),
        tables: schema
            .tables
            .iter()
            .map(|(name, t)| {
                (
                    name.clone(),
                    TableSchema {
                        id: t.id.get(),
                        partition_template: t.partition_template.as_proto().cloned(),
                        columns: t
                            .columns
                            .iter()
                            .map(|(name, c)| {
                                (
                                    name.clone(),
                                    ColumnSchema {
                                        id: c.id.get(),
                                        column_type: c.column_type as i32,
                                    },
                                )
                            })
                            .collect(),
                    },
                )
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_types::{ColumnType, MaxColumnsPerTable, MaxTables};
    use futures::{future::BoxFuture, FutureExt};
    use generated_types::influxdata::iox::{
        partition_template::v1::{template_part, PartitionTemplate, TemplatePart},
        schema::v1::schema_service_server::SchemaService,
    };
    use iox_catalog::{
        interface::RepoCollection,
        mem::MemCatalog,
        test_helpers::{arbitrary_namespace, arbitrary_table},
    };
    use std::sync::Arc;
    use tonic::Code;

    // `SchemaService` has to be specified in this way because the `generated_types` trait is
    // also in scope. Make an alias for convenience and to have one place to explain.
    type Service = super::SchemaService;

    // Given some `catalog_setup` closure that can use the catalog repos and returns a future,
    // set up the catalog, await the future, and return the gRPC service.
    async fn service_setup<S, T>(mut catalog_setup: S) -> Service
    where
        S: (FnMut(&mut dyn RepoCollection) -> BoxFuture<'_, T>) + Send,
    {
        let catalog = Arc::new(MemCatalog::new(Default::default()));
        let mut repos = catalog.repositories().await;

        let setup = catalog_setup(repos.as_mut());
        setup.await;

        Service::new(catalog)
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
                "Namespace `namespace_does_not_exist` not found",
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
                "Tried to upsert column `name` with type i64, \
                but the column is already type string",
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
                "Tried to upsert column `cpu` with type tag, but the column is already type string",
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
                "Table limit reached on namespace `namespace_schema_too_many_tables`; \
                could not upsert table `schema_test_table_upsert`",
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

                    let table = arbitrary_table(&mut *repos, table, &namespace_in_catalog).await;

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
                "Could not create columns in namespace `namespace_schema_too_many_columns`, \
                table `schema_test_table_existing`; table contains 2 existing columns, \
                applying this upsert would result in 3 columns, limit is 2",
            )
            .await;

            let schema = get_schema(&grpc, namespace, None).await;
            assert_eq!(sorted_table_names(&schema), [table]);
            // No new columns should be added
            assert_eq!(sorted_column_names(&schema, table), ["cpu", "time"]);
        }
    }
}
