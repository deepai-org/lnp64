int parent_pid;

int main(){int child;int value;parent_pid=pid();child=fork();if(child==0){value=msg_recv();msg_send(parent_pid,value+1,0);exit(0);}msg_send(child,0,0);value=msg_recv();if(value==1){write(1,"ping pong ok\n",13);return 0;}write(2,"ping pong failed\n",17);return 1;}
