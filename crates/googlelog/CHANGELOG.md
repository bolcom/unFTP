# Changelog

## 0.2.0

- Upgrade `google-logging2` from 5.0.5 to 7.0.0. `Builder::build()` still
  returns a channel of `WriteLogEntriesRequest`, so this is a breaking change
  for callers that ship entries themselves.
- Document that the async shipper authenticates via the GCE metadata server
  as the default service account.

## 0.1.0

- Previous release.
