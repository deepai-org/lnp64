# zlib for LNP64

These files are imported from upstream zlib v1.3.1:

https://github.com/madler/zlib/tree/v1.3.1

The current checked gates cover upstream Adler-32 and CRC-32 checksum paths.
The legacy toy-package gate still builds the local smoke with `zutil.c`; the
real LLVM gate builds `adler32.c` and `crc32.c` plus a generated no-stdio smoke.
The broader compression/decompression surface is tracked in
`conformance_matrix.md` as remaining zlib compatibility work.
