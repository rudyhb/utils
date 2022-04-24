#[macro_export]
macro_rules! compile_warning {(
    $name:ident, $message:expr $(,)*
) => (
    mod $name {
        #[must_use = $message]
        struct CompileWarning;
        #[allow(dead_code, path_statements)]
        fn trigger_warning () { CompileWarning; }
    }
)}