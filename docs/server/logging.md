---
title: Logging
---

By default unFTP will output logs to STD OUT and it will output only Error and Warning messages.

```
➜ unftp
module: main
 Jan 29 12:53:25.187 WARN FTPS not enabled
```

You can have unFTP log at INFO level with the `-v` argument. 

```
➜ unftp -v
module: main
 Jan 29 12:54:17.129 INFO Starting unFTP server., sbe-type: filesystem, auth-type: anonymous, home: /var/folders/dt/tmsf_k596295mkh5md67vb840000gp/T/, http-address: 0.0.0.0:8080, ftp-address: 0.0.0.0:2121, libunftp-version: 0.18.3, version:
 Jan 29 12:54:17.130 INFO Using passive port range 49152..65535
 Jan 29 12:54:17.130 INFO Using passive host option 'FromConnection'
 Jan 29 12:54:17.130 INFO Idle session timeout is set to 600 seconds
 Jan 29 12:54:17.130 INFO Starting HTTP service., address: 0.0.0.0:8080
 Jan 29 12:54:17.130 INFO Exposing unFTP service home., path: /
 Jan 29 12:54:17.130 INFO Exposing Prometheus unFTP exporter endpoint., path: /metrics
 Jan 29 12:54:17.149 INFO Exposing readiness endpoint., path: /ready
 Jan 29 12:54:17.149 INFO Exposing liveness endpoint., path: /health
 Jan 29 12:54:17.149 WARN FTPS not enabled
```

If you want DEBUG level then specify `-vv`.

## Log shipping via Redis

In a cloud environment it is useful to send logs to a central location for analysis. unFTP can send structured logging 
in JSON format to a [Redis](https://redis.io/) instance. With a tool like [Logstash](https://www.elastic.co/logstash/)
this can then be processed further.

unFTP will use the [RPUSH](https://redis.io/commands/rpush) command to append logs at the tail of a list.

Here is an example of configuring unFTP to send logs to a local Redis instance:

```sh
unftp -v \
  --log-redis-host=localhost \
  --log-redis-port=6379 \
  --log-redis-key=logging 
```

The format of the JSON messages look like this:

```json
 {
    "@timestamp": ${timeRFC3339},
    "@source_host": ${hostname},
    "@message": ${message},
    "@fields": {
       "level": ${levelLowercase},
       "application": ${appName}
       ... // logged field 1
       ... // logged field 2
       ...
   }
}
```