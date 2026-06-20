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

int main(int argc, char *argv[]) {
  int port = 8080;
  if (argc > 1)
    port = atoi(argv[1]);

  printf("minimal httpd: listening on port %d\n", port);
  printf("This demonstrates socket bind/listen/accept infrastructure\n");

  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = 2; /* AF_INET */
  addr.sin_port = (unsigned short)port;
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

  printf("Listening for HTTP connections...\n");

  /* Demo: try to accept one connection */
  struct sockaddr_in client_addr;
  unsigned int addr_len = sizeof(client_addr);

  printf("Attempting to accept first connection (with timeout)...\n");
  int client_sock = accept(server_sock, (struct sockaddr *)&client_addr, &addr_len);

  if (client_sock < 0) {
    printf("INFO: accept returned error (errno=%d) - no immediate connections\n", errno);
  } else {
    printf("Accepted HTTP client connection (fd=%d)\n", client_sock);

    /* Send minimal HTTP response */
    const char *http_response =
        "HTTP/1.0 200 OK\r\n"
        "Content-Type: text/plain\r\n"
        "Content-Length: 25\r\n"
        "\r\n"
        "LNP64 Minimal HTTP Server";

    send(client_sock, http_response, strlen(http_response), 0);
    close(client_sock);
  }

  close(server_sock);
  printf("Server shutdown\n");
  printf("exit=0\n");
  return 0;
}
