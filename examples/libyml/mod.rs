/// This module contains the `tag` example.
pub(crate) mod tag_examples;

/// This module contains the `emitter` example.
pub(crate) mod emitter_examples;

/// This module contains the `parser` example.
pub(crate) mod parser_examples;

/// The main function that runs all the example modules.
pub(crate) fn main() {
    // Run the example module `emitter`.
    emitter_examples::main();

    // Run the example module `parser`.
    parser_examples::main();

    // Run the example module `tag`.
    tag_examples::main();
}
