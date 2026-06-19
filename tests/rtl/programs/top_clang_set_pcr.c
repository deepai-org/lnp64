static unsigned long get_tp(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, TP" : "=r"(value) : : "memory");
  return value;
}

static unsigned long get_sigmask(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, SIGMASK" : "=r"(value) : : "memory");
  return value;
}

static unsigned long get_pid(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, PID" : "=r"(value) : : "memory");
  return value;
}

static unsigned long get_cred_profile(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, CRED_PROFILE" : "=r"(value) : : "memory");
  return value;
}

static unsigned long get_cred_handle(void) {
  unsigned long value;
  __asm__ volatile("get_pcr %0, CRED_HANDLE" : "=r"(value) : : "memory");
  return value;
}

static unsigned long set_tp(unsigned long value) {
  unsigned long result;
  __asm__ volatile("set_pcr %0, TP, %1" : "=r"(result) : "r"(value) : "memory");
  return result;
}

static unsigned long set_sigmask(unsigned long value) {
  unsigned long result;
  __asm__ volatile("set_pcr %0, SIGMASK, %1" : "=r"(result) : "r"(value) : "memory");
  return result;
}

static unsigned long set_pid(unsigned long value) {
  unsigned long result;
  __asm__ volatile("set_pcr %0, PID, %1" : "=r"(result) : "r"(value) : "memory");
  return result;
}

static unsigned long set_cred_profile(unsigned long value) {
  unsigned long result;
  __asm__ volatile("set_pcr %0, CRED_PROFILE, %1" : "=r"(result) : "r"(value) : "memory");
  return result;
}

static unsigned long set_cred_handle(unsigned long value) {
  unsigned long result;
  __asm__ volatile("set_pcr %0, CRED_HANDLE, %1" : "=r"(result) : "r"(value) : "memory");
  return result;
}

int main(void) {
  unsigned long status = 0;
  unsigned long tp = 0x1234;
  unsigned long mask = 0x55;

  status |= set_tp(tp);
  status |= get_tp() ^ tp;

  status |= set_sigmask(mask);
  status |= get_sigmask() ^ mask;

  unsigned long pid = get_pid();
  unsigned long cred_profile = get_cred_profile();
  unsigned long cred_handle = get_cred_handle();

  status |= set_pid(tp) ^ ~0ul;
  status |= get_pid() ^ pid;
  status |= set_cred_profile(tp) ^ ~0ul;
  status |= get_cred_profile() ^ cred_profile;
  status |= set_cred_handle(tp) ^ ~0ul;
  status |= get_cred_handle() ^ cred_handle;
  return (int)status;
}
