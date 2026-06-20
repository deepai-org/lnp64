#ifndef LNP64_NETINET_TCP_H
#define LNP64_NETINET_TCP_H

#define TCP_NODELAY   1
#define TCP_MAXSEG    2
#define TCP_CORK      3
#define TCP_KEEPIDLE  4
#define TCP_KEEPINTVL 5
#define TCP_KEEPCNT   6
#define TCP_INFO      11
#define TCP_FASTOPEN  23

struct tcp_info {
  unsigned char  tcpi_state;
  unsigned char  tcpi_ca_state;
  unsigned char  tcpi_retransmits;
  unsigned int   tcpi_rtt;
  unsigned int   tcpi_rttvar;
};

#endif
