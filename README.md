# ReplaceOnce

**A synchronization primitive that allows a value to be replaced _at most once_, safely and efficiently.**

[`ReplaceOnce<T>`](src/lib.rs) is a concurrency-safe alternative to `OnceCell` for cases where a value is known at creation but may be replaced exactly once, non-lazily.

---

## ✨ Features

- ✅ Always initialized with a value
- ✅ Allows a single atomic replacement
- ✅ Returns ownership of replaced/skipped values
- ✅ Safe when used according to documented invariants
- ✅ Thread-safe via `std::sync::Once`

---

## 📦 Add to Your Project

```toml
# Cargo.toml
[dependencies]
replace_once = { git = "https://github.com/partiallyuntyped/replace_once" }
```
