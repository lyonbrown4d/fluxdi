use fluxdi::{
    Application, Error, Injector, Module, Provider, Shared, module::ModuleLifecycleFuture,
};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

#[derive(Debug)]
struct DbUrl(String);

struct SeaOrmSqliteModule;

impl Module for SeaOrmSqliteModule {
    fn configure(&self, injector: &Injector) -> Result<(), Error> {
        injector.provide::<DbUrl>(Provider::root(|_| {
            Shared::new(DbUrl("sqlite::memory:".to_string()))
        }));

        injector.provide::<DatabaseConnection>(Provider::root_async(|inj| async move {
            let db_url = inj.resolve::<DbUrl>();
            let connection = Database::connect(db_url.0.clone())
                .await
                .expect("failed to connect sqlite");
            Shared::new(connection)
        }));

        Ok(())
    }

    fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
        Box::pin(async move {
            let db = injector.try_resolve_async::<DatabaseConnection>().await?;

            db.execute_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT 1".to_string(),
            ))
            .await
            .map_err(|err| {
                Error::module_lifecycle_failed(
                    "SeaOrmSqliteModule",
                    "on_start",
                    &format!("database connectivity check failed: {err}"),
                )
            })?;

            println!("seaorm sqlite connectivity check passed");
            Ok(())
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    fluxdi::init_logging();

    let mut app = Application::new(SeaOrmSqliteModule);
    app.bootstrap().await?;
    app.shutdown().await
}
