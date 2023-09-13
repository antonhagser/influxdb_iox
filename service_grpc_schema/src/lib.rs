//! Implementation of the read-only parts of the schema gRPC service, appropriate for any service
//! other than the router. All schema modifications should go through the router.

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

use std::{ops::DerefMut, sync::Arc};

use generated_types::influxdata::iox::schema::v1::*;
use iox_catalog::interface::{
    get_schema_by_name, get_schema_by_namespace_and_table, Catalog, SoftDeletedRows,
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
        _request: Request<UpsertSchemaRequest>,
    ) -> Result<Response<UpsertSchemaResponse>, Status> {
        Err(Status::unimplemented(
            "Modification operations are only supported by the router",
        ))
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
    use data_types::ColumnType;
    use futures::{future::BoxFuture, FutureExt};
    use generated_types::influxdata::iox::schema::v1::schema_service_server::SchemaService;
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

    #[tokio::test]
    async fn upsert_not_implemented_by_default() {
        let grpc = service_setup(|_repos| async {}.boxed()).await;

        // attempt to upsert, which isn't supported in this implementation
        let request = UpsertSchemaRequest {
            namespace: "arbitrary".to_string(),
            table: "arbitrary".to_string(),
            columns: [("temperature".to_string(), ColumnType::I64 as i32)].into(),
            partition_template: None,
        };

        let status = grpc.upsert_schema(Request::new(request)).await.unwrap_err();
        assert_eq!(status.code(), Code::Unimplemented);
        assert_eq!(
            status.message(),
            "Modification operations are only supported by the router"
        );
    }
}
