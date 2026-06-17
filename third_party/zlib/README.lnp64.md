# zlib for LNP64

These files are imported from upstream zlib v1.3.1:

https://github.com/madler/zlib/tree/v1.3.1

The current checked gate intentionally starts with the upstream Adler-32 path
(`adler32.c` plus `zutil.c`). CRC-32 and the broader compression/decompression
surface are tracked in `conformance_matrix.md` as remaining zlib compatibility
work.
