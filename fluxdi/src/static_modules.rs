//! Statically assembled dependency graphs with explicit binding keys.
//!
//! [`crate::static_module!`] generates typed getters and a constructor taking
//! imports and external values in declaration order. Each sync factory receives the listed
//! dependencies as [`crate::Shared`] values and returns `Result<Shared<T>, Error>`.
//! Bindings can be declared before their dependencies. Different keys may have
//! the same Rust type.
//!
//! ```
//! use fluxdi::{Error, Shared, static_module};
//!
//! fn size(text: Shared<String>) -> Result<Shared<usize>, Error> {
//!     Ok(Shared::new(text.len()))
//! }
//!
//! static_module! {
//!     pub struct App {
//!         size: usize = size(text);
//!         external text: String;
//!     }
//! }
//!
//! let app = App::new(Shared::new("configured".to_owned()));
//! assert_eq!(*app.size()?, 10);
//! assert!(Shared::ptr_eq(&app.size()?, &app.clone().size()?));
//! # Ok::<(), Error>(())
//! ```
//!
//! Import blocks select exports by binding key. `local: Type = source` renames
//! an export; omitting `= source` selects the same key. Module paths may be
//! qualified Rust paths. Imports and external values become constructor
//! arguments in their textual declaration order:
//!
//! ```
//! use fluxdi::{Error, Shared, static_module};
//! fn length(text: Shared<String>) -> Result<Shared<usize>, Error> {
//!     Ok(Shared::new(text.len()))
//! }
//! static_module! {
//!     struct Data {
//!         external text: String;
//!         length: usize = length(text);
//!     }
//! }
//! static_module! {
//!     struct App {
//!         import data: Data { size: usize = length; };
//!         external label: String;
//!     }
//! }
//! let data = Data::new(Shared::new("configured".into()));
//! let app = App::new(data.clone(), Shared::new("app".into()));
//! assert_eq!(*app.size()?, 10);
//! # Ok::<(), Error>(())
//! ```
//!
//! Imports retain selected provider handles, including their original scopes
//! and identity. Reexporting an imported binding preserves these properties.
//! Multiple imports of the same provider share its cache within one context.
//! `new` always creates a fresh root context: it does not adopt an imported
//! module's injector or retain that module's context and unselected bindings.
//! To reuse an existing tree and its caches, use
//! `app.in_context(data.injector())` explicitly. Each source graph is closed;
//! its factories cannot depend on bindings from an importing graph.
//!
//! Async bindings require the `async-factory` feature in addition to `macros`.
//! Declare `async value: Type = factory(dependencies);` when the factory returns
//! a future producing `Result<Shared<Type>, Error>`. A synchronous factory
//! depending on an async binding is automatically promoted: its getter is
//! async, while the factory itself remains synchronous. Promotion propagates
//! through the dependency graph. In an import block, explicitly mark every
//! effectively async export with `async`, including promoted exports.
//!
//! Constructors stay synchronous and lazy. Async getters resolve dependencies
//! sequentially in the factory's argument order and use the same cache,
//! cancellation, and retry rules as typed async provider resolution. Without
//! `thread-safe`, factories may return futures that are not `Send`; with it,
//! factory futures must satisfy the provider's `Send` bound.
//!
//! The default scope is `singleton`, which caches per resolving injector
//! context. `#[scope(root)]` shares across contexts, `#[scope(scoped)]` caches
//! at the nearest runtime scope boundary, and `#[scope(transient)]` creates a
//! fresh value per getter call. `create_child` and `create_scope` retain the
//! typed bindings and create corresponding injector contexts. `in_context`
//! binds those providers to an existing injector context, such as an HTTP
//! request scope.
//!
//! Graph errors are rejected during macro expansion. Unknown dependencies
//! cannot silently fall back to dynamic registry lookup:
//!
//! ```compile_fail
//! use fluxdi::{Error, Shared, static_module};
//! fn make(input: Shared<u32>) -> Result<Shared<u32>, Error> { Ok(input) }
//! static_module! {
//!     struct Missing {
//!         value: u32 = make(absent);
//!     }
//! }
//! ```
//!
//! Binding keys must be unique, even when their factories are identical:
//!
//! ```compile_fail
//! use fluxdi::{Error, Shared, static_module};
//! fn make() -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
//! static_module! {
//!     struct Duplicate {
//!         value: u32 = make();
//!         value: u32 = make();
//!     }
//! }
//! ```
//!
//! Cycles are rejected before any factory runs:
//!
//! ```compile_fail
//! use fluxdi::{Error, Shared, static_module};
//! fn identity(input: Shared<u32>) -> Result<Shared<u32>, Error> { Ok(input) }
//! static_module! {
//!     struct Cycle {
//!         first: u32 = identity(second);
//!         second: u32 = identity(third);
//!         third: u32 = identity(first);
//!     }
//! }
//! ```
//!
//! Rust checks the factory's argument types against the selected bindings:
//!
//! ```compile_fail,E0308
//! use fluxdi::{Error, Shared, static_module};
//! fn make(input: Shared<String>) -> Result<Shared<u32>, Error> {
//!     Ok(Shared::new(input.len() as u32))
//! }
//! static_module! {
//!     struct WrongArgument {
//!         external input: u32;
//!         value: u32 = make(input);
//!     }
//! }
//! ```
//!
//! Rust also checks the fallible shared return type:
//!
//! ```compile_fail,E0308
//! use fluxdi::{Error, Shared, static_module};
//! fn make() -> Result<Shared<String>, Error> { Ok(Shared::new("wrong".into())) }
//! static_module! {
//!     struct WrongReturn {
//!         value: u32 = make();
//!     }
//! }
//! ```
//!
//! Root bindings cannot capture scoped or calling-context singleton bindings,
//! including through transient dependencies:
//!
//! ```compile_fail
//! use fluxdi::{Error, Shared, static_module};
//! fn make() -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
//! fn identity(input: Shared<u32>) -> Result<Shared<u32>, Error> { Ok(input) }
//! static_module! {
//!     struct InvalidLifetime {
//!         #[scope(scoped)]
//!         request: u32 = make();
//!         #[scope(transient)]
//!         intermediate: u32 = identity(request);
//!         #[scope(root)]
//!         application: u32 = identity(intermediate);
//!     }
//! }
//! ```
//!
//! Binding keys cannot shadow generated context methods:
//!
//! ```compile_fail
//! use fluxdi::{Error, Shared, static_module};
//! fn make() -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
//! static_module! {
//!     struct ReservedName {
//!         create_scope: u32 = make();
//!     }
//! }
//! ```
//!
//! Selecting a missing export is a compile error:
//!
//! ```compile_fail,E0599
//! use fluxdi::{Shared, static_module};
//! static_module! { struct Data { external value: u32; } }
//! static_module! {
//!     struct App { import data: Data { missing: u32; }; }
//! }
//! ```
//!
//! Import types must match the selected source binding:
//!
//! ```compile_fail,E0308
//! use fluxdi::{Shared, static_module};
//! static_module! { struct Data { external value: u32; } }
//! static_module! {
//!     struct App { import data: Data { value: String; }; }
//! }
//! ```
//!
//! Root safety follows imported dependencies, including transient source
//! bindings that capture contextual values:
//!
//! ```compile_fail,E0080
//! use fluxdi::{Error, Shared, static_module};
//! fn make() -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
//! fn identity(value: Shared<u32>) -> Result<Shared<u32>, Error> { Ok(value) }
//! static_module! {
//!     struct Data {
//!         #[scope(scoped)] request: u32 = make();
//!         #[scope(transient)] bridge: u32 = identity(request);
//!     }
//! }
//! static_module! {
//!     struct App {
//!         import data: Data { bridge: u32; };
//!         #[scope(root)] root: u32 = identity(bridge);
//!     }
//! }
//! ```
//!
//! Reexports and local transient bridges cannot hide contextual capture:
//!
//! ```compile_fail,E0080
//! use fluxdi::{Error, Shared, static_module};
//! fn make() -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
//! fn identity(value: Shared<u32>) -> Result<Shared<u32>, Error> { Ok(value) }
//! static_module! { struct Data { value: u32 = make(); } }
//! static_module! {
//!     struct Facade { import data: Data { value: u32; }; }
//! }
//! static_module! {
//!     struct App {
//!         import facade: Facade { value: u32; };
//!         #[scope(transient)] bridge: u32 = identity(value);
//!         #[scope(root)] root: u32 = identity(bridge);
//!     }
//! }
//! ```
#![cfg_attr(
    feature = "async-factory",
    doc = r#"
Async source factories and promoted sync factories compose through typed imports:

```
use fluxdi::{Error, Shared, static_module};
async fn load() -> Result<Shared<u32>, Error> { Ok(Shared::new(40)) }
fn increment(value: Shared<u32>) -> Result<Shared<u32>, Error> {
    Ok(Shared::new(*value + 1))
}
static_module! {
    struct Data {
        async value: u32 = load();
        promoted: u32 = increment(value);
    }
}
static_module! {
    struct App {
        import data: Data { async selected: u32 = promoted; };
        result: u32 = increment(selected);
    }
}
let app = App::new(Data::new());
assert_eq!(*futures::executor::block_on(app.result())?, 42);
# Ok::<(), Error>(())
```

Async metadata is checked in both directions. A promoted source export must
be imported as async:

```compile_fail,E0080
use fluxdi::{Error, Shared, static_module};
async fn make() -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
fn identity(value: Shared<u32>) -> Result<Shared<u32>, Error> { Ok(value) }
static_module! {
    struct Data {
        async value: u32 = make();
        promoted: u32 = identity(value);
    }
}
static_module! {
    struct App { import data: Data { promoted: u32; }; }
}
```

A synchronous source export cannot be marked async in an import:

```compile_fail,E0080
use fluxdi::{Shared, static_module};
static_module! { struct Data { external value: u32; } }
static_module! {
    struct App { import data: Data { async value: u32; }; }
}
```
"#
)]
#![cfg_attr(
    all(feature = "async-factory", feature = "thread-safe"),
    doc = r#"
With `thread-safe`, a factory cannot hold a non-`Send` value across suspension:

```compile_fail
use fluxdi::{Error, Shared, static_module};
async fn local() -> Result<Shared<u32>, Error> {
    let value = std::rc::Rc::new(1);
    std::future::pending::<()>().await;
    Ok(Shared::new(*value))
}
static_module! { struct App { async value: u32 = local(); } }
```
"#
)]
#![cfg_attr(
    not(feature = "async-factory"),
    doc = r#"
Async declarations require enabling the `async-factory` feature:

```compile_fail
use fluxdi::{Error, Shared, static_module};
async fn make() -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
static_module! { struct App { async value: u32 = make(); } }
```
"#
)]
// Collection and optional edges are checked just like required edges, after
// compiler configuration has selected the active declarations.
#![doc = r#"
Optional dependencies and collection members participate in cycle checks:

```compile_fail
use fluxdi::{Error, Shared, static_module};
fn make(_: Option<Shared<u32>>) -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
static_module! { struct OptionalCycle { value: u32 = make(optional(value)); } }
```

```compile_fail
use fluxdi::{Error, Shared, static_module};
fn make(_: Vec<Shared<u32>>) -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
static_module! {
    struct CollectionCycle {
        #[set(values)] value: u32 = make(all(values));
    }
}
```

Root bindings cannot capture contextual values through either argument form:

```compile_fail
use fluxdi::{Error, Shared, static_module};
fn value() -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
fn make(_: Option<Shared<u32>>) -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
static_module! {
    struct OptionalCapture {
        #[scope(scoped)] request: u32 = value();
        #[scope(root)] root: u32 = make(optional(request));
    }
}
```

An expanded collection retains diagnostic locations even after earlier arguments
and multiple members have added graph edges:

```compile_fail
use fluxdi::{Error, Shared, static_module};
fn value() -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
fn make(_: Shared<u32>, _: Vec<Shared<u32>>, _: Option<Shared<u32>>)
    -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
static_module! {
    struct CollectionCapture {
        #[scope(root)] safe: u32 = value();
        #[scope(root)] #[set(values)] first: u32 = value();
        #[scope(root)] #[set(values)] second: u32 = value();
        #[scope(scoped)] #[set(values)] request: u32 = value();
        #[scope(root)] root: u32 = make(safe, all(values), optional(absent));
    }
}
```

Imported contextual capture is checked through collections too:

```compile_fail,E0080
use fluxdi::{Error, Shared, static_module};
fn value() -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
fn make(_: Vec<Shared<u32>>) -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
static_module! { struct Data { value: u32 = value(); } }
static_module! {
    struct App {
        import data: Data { #[set(values)] value: u32; };
        #[scope(root)] root: u32 = make(all(values));
    }
}
```

Rust checks optional and collection element types:

```compile_fail,E0308
use fluxdi::{Error, Shared, static_module};
fn make(_: Option<Shared<String>>) -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
static_module! {
    struct WrongOptional {
        external value: u32;
        result: u32 = make(optional(value));
    }
}
```

```compile_fail,E0308
use fluxdi::{Error, Shared, static_module};
fn make(_: Vec<Shared<String>>) -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
static_module! {
    struct WrongCollection {
        #[set(values)] external value: u32;
        result: u32 = make(all(values));
    }
}
```

Membership lists must be nonempty and must not repeat the same group:

```compile_fail
use fluxdi::static_module;
static_module! { struct EmptyMembership { #[set()] external value: u32; } }
```

```compile_fail
use fluxdi::static_module;
static_module! {
    struct DuplicateMembership {
        #[set(values)] #[set(r#values)] external value: u32;
    }
}
```

Only a binding key or a single-level `optional(key)` / `all(group)` argument
is accepted:

```compile_fail
use fluxdi::{Error, Shared, static_module};
fn make(_: Option<Shared<u32>>) -> Result<Shared<u32>, Error> { Ok(Shared::new(1)) }
static_module! {
    struct NestedArgument { result: u32 = make(optional(optional(value))); }
}
```
"#]
