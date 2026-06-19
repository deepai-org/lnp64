static unsigned long get_pid(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, PID" : "=r"(value) : : "memory");
  return value;
}

static unsigned long get_tid(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, TID" : "=r"(value) : : "memory");
  return value;
}

static unsigned long get_tp(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, TP" : "=r"(value) : : "memory");
  return value;
}

int main(void) {
  unsigned long status = 0;
  status |= get_pid() ^ 1;
  status |= get_tid() ^ 1;
  status |= get_tp();
  return (int)status;
}
