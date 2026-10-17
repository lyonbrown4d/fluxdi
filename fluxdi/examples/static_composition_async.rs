use fluxdi::{Error, Shared, static_module};

struct Config(&'static str);
struct Database(Shared<Config>);
struct Repository(Shared<Database>);
struct Service(Shared<Repository>);

async fn connect(config: Shared<Config>) -> Result<Shared<Database>, Error> {
    // An application can await its database client's connection future here.
    Ok(Shared::new(Database(config)))
}

fn repository(database: Shared<Database>) -> Result<Shared<Repository>, Error> {
    Ok(Shared::new(Repository(database)))
}

fn service(repository: Shared<Repository>) -> Result<Shared<Service>, Error> {
    Ok(Shared::new(Service(repository)))
}

mod persistence {
    use super::*;

    static_module! {
        pub(super) struct Persistence {
            #[scope(root)]
            async database: Database = connect(config);
            #[scope(root)]
            repository: Repository = repository(database);
            external config: Config;
        }
    }
}

static_module! {
    struct App {
        import persistence: persistence::Persistence {
            async repository: Repository;
        };
        #[scope(scoped)]
        service: Service = service(repository);
    }
}

fn main() -> Result<(), Error> {
    futures::executor::block_on(async {
        let persistence =
            persistence::Persistence::new(Shared::new(Config("memory://composition")));
        let app = App::new(persistence);
        let request = app.create_scope();
        let first = request.service().await?;
        let second = request.service().await?;
        assert!(Shared::ptr_eq(&first, &second));
        println!("{}", first.0.0.0.0);
        Ok(())
    })
}
