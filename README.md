# ReplaceOnce

**A synchronization primitive that allows a value to be replaced _at most once_, safely and efficiently.**

[`ReplaceOnce<T>`](src/lib.rs) is a concurrency-safe alternative to `OnceCell` for cases where a value is known at creation but may be replaced exactly once, non-lazily.


## ✨ Features

- ✅ Always initialized with a value
- ✅ Allows a single atomic replacement
- ✅ Returns ownership of replaced/skipped values
- ✅ Safe when used according to documented invariants
- ✅ Thread-safe via `std::sync::Once`


## 📦 Add to Your Project

```toml
# Cargo.toml
[dependencies]
replace_once = { git = "https://github.com/partiallyuntyped/replace_once" }
```


## 🚀 Example
```rust
use replace_once::{ReplaceOnce, ReplaceResult};

let cell = ReplaceOnce::new(42);
assert_eq!(*cell.get(), 42);

let replaced = cell.replace(100);
assert_eq!(replaced, ReplaceResult::Replaced(42));
assert_eq!(*cell.get(), 100);

let skipped = cell.replace(200);
assert_eq!(skipped, ReplaceResult::Skipped(200));
assert_eq!(*cell.get(), 100);
```


## 🧠 When to Use This

Use `ReplaceOnce<T>` when:

- You need a value at creation time, but might want to replace it exactly once later.

- You don’t want lazy initialization like `OnceCell` or `Lazy`.

- You want clear, ownership-aware APIs with success/failure semantics on replacement.

- You want to synchronize across threads, safely.


## ⚠️ Safety

Internally, `ReplaceOnce` uses `UnsafeCell` and `Once`. This makes it sound but unsafe if misused.
You must ensure:

- No simultaneous access while a replacement is occurring.

- No mutable references exist while calling .get().

- You don’t create aliasing by holding multiple &T or &mut T in unsafe ways.

These requirements are upheld automatically in typical usage, but must be respected in low-level or concurrent contexts.


## 📖 API Overview

|Method |	Description|
|:-|-:|
`replace(t)` |	Replace the value with t once, return old or skipped
`replace_with(f)` |	Replace using closure
`get()`|	Get a reference to the value
`get_or_replace(t)`|	Replace-or-return reference + ReplaceResult<T>
`get_or_replace_with(f)`|	Same, but lazy
`has_been_replaced()`|	Whether a replacement has occurred
`wait()`|	Wait until replacement (or original value is visible)



## 🔒 Thread Safety

`ReplaceOnce<T>` is `Send` and `Sync` if `T` is. Internally, `std::sync::Once` ensures the replacement operation is safe and race-free — as long as no **unsynchronized access occurs around it.**


## 🤝 Contributing

PRs, issues, and suggestions welcome.

## ❤️ Inspired By
- `OnceCell`
- `std::sync::Once`