//! Injectable derive macro example.
//!
//! This example mirrors `examples/basic` using `#[derive(Injectable)]` instead
//! of manual provider closures. Both approaches produce equivalent behavior.

use fluxdi::{Injectable, Injector, Provider, Shared};

// ============================================================
// SERVICE TYPES (same as basic example)
// ============================================================

#[derive(Clone, Debug)]
struct Config {
    database_url: String,
    environment: String,
}

impl Config {
    fn new() -> Self {
        Self {
            database_url: "postgresql://localhost/mydb".to_string(),
            environment: "development".to_string(),
        }
    }
}

#[derive(Debug)]
struct Logger {
    prefix: String,
}

impl Logger {
    fn new(config: &Config) -> Self {
        Self {
            prefix: format!("[{}]", config.environment),
        }
    }

    fn log(&self, message: &str) {
        println!("{} {}", self.prefix, message);
    }
}

#[derive(Debug)]
struct Database {
    _url: String,
    id: usize,
}

impl Database {
    fn new(config: &Config) -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self {
            _url: config.database_url.clone(),
            id,
        }
    }

    fn query(&self, sql: &str) -> String {
        format!("DB[{}]: {}", self.id, sql)
    }
}

// Injectable: struct with only Shared<T> fields
#[derive(Injectable, Debug)]
struct AppService {
    db: Shared<Database>,
    logger: Shared<Logger>,
}

impl AppService {
    fn run_query(&self, sql: &str) -> String {
        self.logger.log(&format!("Executing: {}", sql));
        self.db.query(sql)
    }
}

// ============================================================
// MAIN
// ============================================================

fn main() {
    fluxdi::init_logging();

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║     FluxDI - Injectable Derive Macro Example           ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    let injector = Injector::root();

    // Config and Logger: no Shared deps, use manual providers
    injector
        .try_provide::<Config>(Provider::singleton(|_| Shared::new(Config::new())))
        .expect("register Config");
    injector
        .try_provide::<Logger>(Provider::singleton(|inj| {
            let config = inj.try_resolve::<Config>().expect("resolve Config");
            Shared::new(Logger::new(&config))
        }))
        .expect("register Logger");
    injector
        .try_provide::<Database>(Provider::singleton(|inj| {
            let config = inj.try_resolve::<Config>().expect("resolve Config");
            Shared::new(Database::new(&config))
        }))
        .expect("register Database");

    // AppService: uses #[derive(Injectable)] - one line instead of manual closure
    injector
        .try_provide::<AppService>(AppService::provider(fluxdi::Scope::Root))
        .expect("register AppService");

    injector
        .try_validate_graph()
        .expect("validate derived dependency hints");
    let app = injector.resolve::<AppService>();
    let result = app.run_query("SELECT * FROM users");
    println!("Result: {}\n", result);

    println!("✓ Injectable macro reduces boilerplate for Shared<T> dependencies.");
}
