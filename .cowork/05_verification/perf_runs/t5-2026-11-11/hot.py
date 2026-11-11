import sys
for f in sys.argv[1:]:
    first={};last={};nm={}
    for l in open(f):
        t,tid,n,s,tk,u,k=l.split();t=float(t);nm[tid]=n
        first.setdefault(tid,(t,int(tk),int(u),int(k))); last[tid]=(t,int(tk),int(u),int(k))
    for role in ("dot-logger-io","dot-logger-host"):
        th=[t for t in last if nm[t]==role]
        hot=sorted(th,key=lambda t:-(last[t][1]-first[t][1]))[: (4 if role.endswith("io") else 1)]
        u=sum(last[t][2]-first[t][2] for t in hot)/100; k=sum(last[t][3]-first[t][3] for t in hot)/100
        print(f"{f:28s} {role:16s} x{len(hot)} user/GiB {u/11.9:.2f} sys/GiB {k/11.9:.2f}")
