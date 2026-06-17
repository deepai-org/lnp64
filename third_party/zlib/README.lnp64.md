# zlib for LNP64

These files are imported from upstream zlib v1.3.1:

https://github.com/madler/zlib/tree/v1.3.1

The current checked gate covers upstream Adler-32 and CRC-32 checksum paths
(`adler32.c`, `crc32.c`, `crc32.h`, and `zutil.c`). The broader
compression/decompression surface is tracked in `conformance_matrix.md` as
remaining zlib compatibility work.
