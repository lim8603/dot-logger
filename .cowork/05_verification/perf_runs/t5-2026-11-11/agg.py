import sys, collections
mx={}; name={}; st=collections.defaultdict(collections.Counter); ts=set(); first={}
for line in open(sys.argv[1]):
    t,tid,n,s,ticks,u,k=line.split(); t=float(t); ts.add(t)
    mx[tid]=(int(ticks),int(u),int(k)); name[tid]=n; st[n][s]+=1
    first.setdefault(tid,t)
g=collections.defaultdict(lambda:[0,0,0,0])
for tid,(tk,u,k) in mx.items():
    x=g[name[tid]]; x[0]+=tk; x[1]+=u; x[2]+=k; x[3]+=1
for n,(tk,u,k,c) in sorted(g.items(), key=lambda kv:-kv[1][0]):
    s=st[n]; ss=sum(s.values())
    print(f"{n:18s} threads {c:3d} cpu {tk/100:6.2f}s user {u/100:6.2f} sys {k/100:6.2f}  states "+" ".join(f"{a}:{b*100//ss}%" for a,b in s.most_common()))
