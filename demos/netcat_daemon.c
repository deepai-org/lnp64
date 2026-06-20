#include <lnp64/intrinsics.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>

void print_hex(const unsigned char *data, int len) {
  for (int i = 0; i < len; i++)
    printf("%02x ", data[i]);
  printf("\n");
}

int main(int argc, char *argv[]) {
  int port = 9999;
  if (argc > 1)
    port = atoi(argv[1]);

  printf("netcat-style daemon: listening on port %d\n", port);
  printf("This demonstrates socket infrastructure readiness\n");

  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = 2; /* AF_INET */
  addr.sin_port = (unsigned short)port; /* Network byte order handled by kernel */
  addr.sin_addr.s_addr = 0; /* INADDR_ANY */

  int server_sock = socket(2, 1, 0); /* AF_INET, SOCK_STREAM, 0 */
  if (server_sock < 0) {
    printf("ERROR: socket creation failed\n");
    return 1;
  }

  printf("Socket created: fd=%d\n", server_sock);

  int reuse = 1;
  setsockopt(server_sock, 1, 2, &reuse, sizeof(reuse)); /* SOL_SOCKET, SO_REUSEADDR */

  int bind_result = bind(server_sock, (struct sockaddr *)&addr, sizeof(addr));
  if (bind_result < 0) {
    printf("ERROR: bind failed (errno=%d)\n", errno);
    close(server_sock);
    return 1;
  }

  printf("Bound to port %d\n", port);

  int listen_result = listen(server_sock, 5);
  if (listen_result < 0) {
    printf("ERROR: listen failed\n");
    close(server_sock);
    return 1;
  }

  printf("Listening for connections...\n");

  /* Nonblocking mode would require O_NONBLOCK support */
  int flags = fcntl(server_sock, 3, 0); /* F_GETFL */
  fcntl(server_sock, 4, flags | 0x800); /* F_SETFL, O_NONBLOCK */
  printf("Nonblocking mode set\n");

  /* For this demo, just accept one connection and echo the data */
  struct sockaddr_in client_addr;
  unsigned int addr_len = sizeof(client_addr);

  int client_sock = accept(server_sock, (struct sockaddr *)&client_addr, &addr_len);
  if (client_sock < 0) {
    printf("INFO: accept would block or error (errno=%d)\n", errno);
  } else {
    printf("Accepted connection from client (fd=%d)\n", client_sock);

    /* Echo back some data */
    const char *response = "netcat-lnp64\n";
    send(client_sock, response, strlen(response), 0);

    close(client_sock);
  }

  close(server_sock);
  printf("Daemon shutdown\n");
  printf("exit=0\n");
  return 0;
}
