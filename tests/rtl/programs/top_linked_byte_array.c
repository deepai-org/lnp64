int main(void) {
  volatile unsigned char buf[4];
  buf[0] = 65;
  buf[1] = 2;
  buf[2] = 7;
  buf[3] = 0;

  return (buf[0] + buf[1] + buf[2] + buf[3]) == 74 ? 0 : 1;
}
