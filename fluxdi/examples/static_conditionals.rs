use fluxdi::{Error, Shared, static_module};

trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
}

struct NamedPlugin(&'static str);
impl Plugin for NamedPlugin {
    fn name(&self) -> &'static str {
        self.0
    }
}

struct Config;
struct Service {
    config: Option<Shared<Config>>,
    plugins: Vec<Shared<dyn Plugin>>,
}

fn plugin() -> Result<Shared<dyn Plugin>, Error> {
    Ok(Shared::new(NamedPlugin("base")))
}

#[cfg(feature = "debug")]
fn debug_plugin() -> Result<Shared<dyn Plugin>, Error> {
    Ok(Shared::new(NamedPlugin("debug")))
}

#[cfg(feature = "debug")]
fn config() -> Result<Shared<Config>, Error> {
    Ok(Shared::new(Config))
}

fn service(
    config: Option<Shared<Config>>,
    plugins: Vec<Shared<dyn Plugin>>,
) -> Result<Shared<Service>, Error> {
    Ok(Shared::new(Service { config, plugins }))
}

static_module! {
    struct App {
        #[scope(root)]
        #[set(plugins)]
        base: dyn Plugin = plugin();
        #[cfg(feature = "debug")]
        #[scope(root)]
        #[set(plugins)]
        debug: dyn Plugin = debug_plugin();
        #[cfg(feature = "debug")]
        #[scope(root)]
        config: Config = config();
        service: Service = service(optional(config), all(plugins));
    }
}

fn main() -> Result<(), Error> {
    let app = App::new();
    let service = app.service()?;
    let names: Vec<_> = service.plugins.iter().map(|plugin| plugin.name()).collect();
    assert_eq!(service.config.is_some(), cfg!(feature = "debug"));
    assert_eq!(names.len(), if cfg!(feature = "debug") { 2 } else { 1 });
    println!("config={}, plugins={names:?}", service.config.is_some());
    Ok(())
}
