/* memory.x - Linker script for the STM32F407G */
MEMORY
{
  /* NOTE K = KiBi = 1024 bytes */
  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K 
  RAM : ORIGIN = 0x20000000, LENGTH = 128K
}

/* _stack_start = ORIGIN(RAM) + LENGTH(RAM); */