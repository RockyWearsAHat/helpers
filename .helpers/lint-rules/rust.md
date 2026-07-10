# Rust law for this repo

## no_dbg [high]
Never leave `dbg!` in committed Rust code; delete the probe before committing.

## no_todo_macro [high]
Never leave `todo!` in committed code; implement the behavior or return a typed error.

## no_unimplemented [high]
Never leave `unimplemented!` in committed code; implement the behavior or return a typed error.
