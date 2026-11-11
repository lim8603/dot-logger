import sys,collections
# per-thread busy in the middle 60% of the hot IO threads' active span
for f in sys.argv[1:]:
    rows=collections.defaultdict(list);nm={}
    for l in open(f):
        t,tid,n,s,tk,u,k=l.split();rows[tid].append((float(t),int(tk)));nm[tid]=n
    io=[t for t in rows if nm[t]=="dot-logger-io"]
    hot=sorted(io,key=lambda t:-(rows[t][-1][1]-rows[t][0][1]))[:4]
    # active span: from first tick increase to last tick increase of hot threads
    def span(r):
        inc=[r[i][0] for i in range(1,len(r)) if r[i][1]>r[i-1][1]]
        return inc[0],inc[-1]
    a=min(span(rows[t])[0] for t in hot); b=max(span(rows[t])[1] for t in hot)
    lo,hi=a+0.2*(b-a),a+0.8*(b-a)
    out=[]
    for t in io+[x for x in rows if nm[x]=="dot-logger-host"]:
        r=[x for x in rows[t] if lo<=x[0]<=hi]
        if len(r)>1: out.append((nm[t][11:],(r[-1][1]-r[0][1])/100/(r[-1][0]-r[0][0])))
    print(f, f"active {b-a:.1f}s mid-window busy:", " ".join(f"{n}:{v*100:.0f}%" for n,v in sorted(out,key=lambda x:-x[1])))
