# evtx2bodyfile
Parses a lot of evtx files and prints a bodyfile

# Usage

```shell
# convert to bodyfile only
evtx2bodyfile Security.evtx >Security.bodyfile

# create a complete timeline
evtx2bodyfile *.evtx | mactime2 -d -b >evtx_timeline.csv
```
