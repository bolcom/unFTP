---
title: Logging
---

This page covers how to configure logging in unFTP, including log levels, structured logging, and shipping logs to external services like Google Cloud Logging or Redis.

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

## Log shipping

In a cloud environment it is useful to send logs to a central location for analysis.
UnFTP can send structured logging to [Google Logging](https://cloud.google.com/logging/docs/) or Redis.
The Redis option is now deprecated and will be soon phased out.

### Google Logging

unFTP can log to Google Cloud Logging.

Minimal settings are _logname_ (`projects/[PROJECT_ID]/logs/[LOG_ID]`) and [_resource type_](https://cloud.google.com/logging/docs/api/v2/resource-list#resource-types).
These can be set through the environment variables `UNFTP_GLOG_LOGNAME` and `UNFTP_GLOG_RESOURCE_TYPE` or the below command line arguments:.


```
➜ unftp \
   --log-google-logname projects/my-gcp-project/logs/my-log-id \
   --log-google-resource-type k8s_container
```

Authentication works with [Application Default Credentials (ADC)](https://cloud.google.com/docs/authentication/application-default-credentials).

Refer to the CLI help for additional optional arguments, including configuring extra labels and specifying a log level label.

### Redis

unFTP can send structured logging in JSON format to a [Redis](https://redis.io/) instance.
With a tool like [Logstash](https://www.elastic.co/logstash/) this can then be processed further.

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

Now that we've covered logging, you may want to set up [monitoring with Prometheus](/server/monitoring) or configure [cloud storage backends](/server/cloud-storage).

