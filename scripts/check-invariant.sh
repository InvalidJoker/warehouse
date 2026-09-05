#!/usr/bin/env bash
# Fails if a schema type gained a field that could carry a fetchable location.
#
# Catalogs carry version identifiers only. Upstream URLs belong in the resolver crate,
# where they are constants this project controls; they must never reach a document a
# consumer reads. See crates/warehouse-types/src/lib.rs for why.
set -euo pipefail

schema_dir="warehouse_common/src/types"
status=0

# URL literals anywhere in the schema crate, outside doc comments.
if grep -rnE '"[a-z]+://' "$schema_dir" | grep -vE '^\s*[^:]+:[0-9]+:\s*(///|//!|//)'; then
  echo "error: a URL literal appears in the schema crate" >&2
  status=1
fi

# Location-shaped field names on schema types.
if grep -rniE '^\s+pub [a-z0-9_]*(url|uri|href|link|image|download|mirror|checksum)[a-z0-9_]*\s*:' "$schema_dir"; then
  echo "error: a schema type declares a location-shaped field" >&2
  status=1
fi

if [ "$status" -eq 0 ]; then
  echo "identifier invariant: ok"
fi
exit "$status"
