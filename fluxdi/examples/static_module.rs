use fluxdi::{Error, Shared, static_module};

trait Repository: fluxdi::Service {
    fn location(&self) -> &str;
}

struct MemoryRepository(Shared<String>);

impl Repository for MemoryRepository {
    fn location(&self) -> &str {
        &self.0
    }
}

struct Service(Shared<dyn Repository>);

fn repository(config: Shared<String>) -> Result<Shared<dyn Repository>, Error> {
    Ok(Shared::new(MemoryRepository(config)))
}

fn service(repository: Shared<dyn Repository>) -> Result<Shared<Service>, Error> {
    Ok(Shared::new(Service(repository)))
}

static_module! {
    struct App {
        #[scope(scoped)]
        service: Service = service(repository);
        #[scope(root)]
        repository: dyn Repository = repository(config);
        external config: String;
    }
}

fn main() -> Result<(), Error> {
    let app = App::new(Shared::new("memory://demo".to_owned()));
    let first = app.create_scope();
    let second = app.create_scope();
    assert!(Shared::ptr_eq(&first.service()?, &first.service()?));
    assert!(!Shared::ptr_eq(&first.service()?, &second.service()?));
    assert!(Shared::ptr_eq(&first.repository()?, &second.repository()?));
    println!("{}", first.service()?.0.location());
    Ok(())
}
