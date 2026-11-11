#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#define SZ (8u<<20)
static int iters=200;
static double now(){struct timespec t;clock_gettime(CLOCK_MONOTONIC,&t);return t.tv_sec+t.tv_nsec*1e-9;}
static void*run(void*a){char*s=malloc(SZ),*d=malloc(SZ);memset(s,1,SZ);memset(d,2,SZ);long x=0;for(int i=0;i<iters;i++){memcpy(d,s,SZ);__asm__ volatile(""::"r"(d):"memory");x+=d[i*4097%SZ];s[i]++;}free(s);free(d);return (void*)x;}
int main(int c,char**v){int n=atoi(v[1]);pthread_t t[64];double t0=now();for(int i=0;i<n;i++)pthread_create(&t[i],0,run,0);for(int i=0;i<n;i++)pthread_join(t[i],0);double dt=now()-t0;printf("threads %2d memcpy %.1f GB/s (copied bytes)\n",n,(double)n*iters*SZ/dt/1e9);}
