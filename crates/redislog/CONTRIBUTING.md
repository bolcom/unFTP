# Contributing to slog-redis

Great projects are never established alone. Therefore we thank you for your interest to help out and offer this guide to
help you along.

- For bug reports you can log a [Github issue](https://github.com/bolcom/unFTP/issues)
- For ideas and feedback you can proceed to the [slog-redis general discussion](https://github.com/bolcom/unFTP/discussions/83)
- Off course pull requests for code and documentation improvements are welcomed.

## Submitting bug reports and feature requests

When reporting a bug or asking for help, please include enough details so that the people helping you can reproduce the
behavior you are seeing. For some tips on how to approach this, read about how to produce
a [Minimal, Complete, and Verifiable example](https://stackoverflow.com/help/mcve).

When making a feature request, please make it clear what problem you intend to solve with the feature, any ideas for how unFTP could support solving that problem, any possible alternatives, and any disadvantages.

## Checking your code

We encourage you to check that the test suite passes locally and make sure that clippy and rustfmt are happy before
submitting a pull request with your changes. If anything does not pass, typically it will be easier to iterate and
fix it locally than waiting for the CI servers to run tests for you. Pull requests that do not pass the CI pipeline
will unfortunately not be merged.

To make sure it will pass please run the following before you submit your pull request:

```sh
cargo fmt --all
cargo clippy
cargo build --all
cargo test --all
cargo doc --no-deps
```

For your convenience we've added a makefile target to check if you got it right. Simply run `make pr-prep`.