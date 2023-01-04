# tapcp_rs

A rust implementation of the [TAPCP](https://github.com/casper-astro/mlib_devel/blob/m2021a/jasper_library/sw/jam/casper_tapcp.c) protocol for interacting with certain CASPER Collaboration FPGA boards.

Currently not quite feature complete, mainly missing interactions with the flash memory.

## TODO

Reimplement CSL in rust so we don't have to rely on an external C compiler and unsafe FFI

## Why does this include an implementation of TFTP

I couldn't find a TFTP client crate and it seemed easy enough with "canonical" implementations only ~300 lines of C.

## Talking to remote TAPCP Client

Unrelated to the details of this package, I have found an *easy* way to interact with remote TFTP clients, including those running TAPCP. The problem is that TFTP runs over UDP, so you can't use an SSH proxy to access it. To solve this, you could use a VPN or a nice piece of software called [sshuttle](https://github.com/sshuttle/sshuttle). Using the TPROXY mode, you can proxy all of the traffic on a given subnet to your remote machine without admin access on the server.

For example, I use this command to test this package:

On first boot (on the client), I run
```shell
ip route add local default dev lo table 100
ip rule add fwmark 0x01 lookup 100
ip -6 route add local default dev lo table 100
ip -6 rule add fwmark 0x01 lookup 100
```

Then to turn on the proxy to the 192.168.0.x subnet, I do:

```shell
sudo sshuttle --method=tproxy -t 0x01 -r <username@server-addr> 192.168.0.0/24
```

## TODO

Make nostd so we can run this on an FPGA softcore at some point