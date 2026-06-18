/* LNP64 static executable linker script v0. */
OUTPUT_ARCH(lnp64)
ENTRY(_start)

PHDRS
{
  text PT_LOAD FLAGS(5);
  rodata PT_LOAD FLAGS(4);
  data PT_LOAD FLAGS(6);
  tls PT_TLS FLAGS(4);
  note PT_NOTE FLAGS(4);
}

SECTIONS
{
  . = 0x400000;
  __lnp64_image_base = .;

  .text : ALIGN(0x1000)
  {
    KEEP(*(.text.startup .text.startup.*))
    *(.text .text.*)
  } :text

  .rodata : ALIGN(0x1000)
  {
    *(.rodata .rodata.*)
    *(.eh_frame .eh_frame.*)
    *(.gcc_except_table .gcc_except_table.*)
  } :rodata

  .note.lnp64.startup : ALIGN(8)
  {
    KEEP(*(.note.lnp64.startup))
  } :note

  .note.lnp64.capreq : ALIGN(8)
  {
    KEEP(*(.note.lnp64.capreq))
  } :note

  .data : ALIGN(0x1000)
  {
    *(.data .data.*)
    *(.sdata .sdata.*)
  } :data

  .tdata : ALIGN(16)
  {
    __lnp64_tls_start = .;
    *(.tdata .tdata.*)
  } :data :tls

  .tbss (NOLOAD) : ALIGN(16)
  {
    *(.tbss .tbss.*)
    __lnp64_tls_end = .;
  } :data :tls

  .bss (NOLOAD) : ALIGN(0x1000)
  {
    *(.bss .bss.*)
    *(COMMON)
  } :data

  __lnp64_image_end = .;

  /DISCARD/ :
  {
    *(.comment)
    *(.note.GNU-stack)
  }
}
