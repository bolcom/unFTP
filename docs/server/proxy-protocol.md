---
title: Proxy Protocol
---

This page explains how to configure unFTP to run behind a proxy server using the proxy protocol, which preserves client IP addresses when running behind load balancers or reverse proxies.

# Proxy Protocol Support

Run behind a proxy in [proxy protocol](https://www.haproxy.com/blog/haproxy/proxy-protocol/) mode:

```sh
unftp \
    --proxy-external-control-port=2121
```

Now that we've covered proxy protocol support, you may want to explore [Docker deployment options](/server/docker) or configure [monitoring with Prometheus](/server/monitoring).
