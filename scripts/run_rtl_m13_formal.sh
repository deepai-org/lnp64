#!/usr/bin/env bash
# Back-compat shim: M13 PCIe/IOMMU formal gate now runs via the generic driver.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec bash "$root/scripts/run_rtl_formal.sh" m13
