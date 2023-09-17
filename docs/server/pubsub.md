---
title: Pub/Sub
---

For systems working alongside unFTP it might be useful to know of FTP related events happening. For this unFTP supports
integration with [Google Pub/Sub](https://cloud.google.com/pubsub).

# Enabling the Pub/Sub integration

You can enable the unFTP Pub/Sub notifier by specifying the `--ntf-pubsub-project` and `--ntf-pubsub-topic` arguments. 

NOTE: Currently authentication with [workload identity](https://cloud.google.com/kubernetes-engine/docs/how-to/workload-identity) is the 
only supported authentication mechanism.

Here is an example:

```sh
unftp \
  --ntf-pubsub-project="my-project" \
  --ntf-pubsub-topic="unftp-events"
```

# The Message Format

The Pub/Sub message sent by unFTP contains meta data (attributes) as shown below and a body in JSON format.

## Message attributes:

Message attributes can be used to [filter messages](https://cloud.google.com/pubsub/docs/filtering) messages such that
when you receive messages from a subscription with a filter, you only receive the messages that match the filter. unFTP
only specifies an `eventType` attribute:

Key              | Value  | Description |
-----------------|--------|-----|
eventType        | _One of_: <p/>- startup <br/> - login <br/>- logout <br/>- get <br/>- put <br/>- delete <br/>- makeDir <br/>- rename <br/>- removeDir | Indicates the type of event |

## Message Body

The message body is a JSON object with these fields:

| Field           | Type           | Explanation                                                                                          |
|-----------------|----------------|------------------------------------------------------------------------------------------------------|
| source_instance | string         | This is the name of the unFTP instance as set by the `--instance-name` variable. Default is 'unFTP'. |
| hostname        | string         | The operating system host name where unFTP is running.                                               |
| payload         | Payload Object | More detail on the specific event type. See below.                                                   |
| username        | string         | The name used during FTP login or "unknown" if not logged in yet.                                    |
| trace_id        | string         | A number that uniquely identifies the FTP connection or session.                                     |
| sequence_number | number         | Identifies the position of the event in the sequence of events for the connection.                   |

The `Payload Object` (payload field) can be one of:

- Startup
- Login
- Logout
- Get
- Put
- Delete
- MakeDir
- Rename
- RemoveDir

All of them are of type JSON object. Examples of their format are shown below.

### Example Events

**Startup Event:**

```json
{
   "source_instance":"unFTP",
   "hostname":"MYMAC-XYZ",
   "payload":{
      "Startup":{
         "libunftp_version":"0.19.0",
         "unftp_version":"v0.14.4"
      }
   }
}
```

**Login Event:**

```json
{
   "source_instance":"unFTP",
   "hostname":"MYMAC-XYZ",
   "payload":{
      "Login":{ }
   },
   "username":"hannes",
   "trace_id":"0xe25ceb1d960303f3",
   "sequence_number":1
}
```

**Logout Event:**

```json
{
   "source_instance":"unFTP",
   "hostname":"MYMAC-XYZ",
   "payload":{
      "Logout":{ }
   },
   "username":"hannes",
   "trace_id":"0xe25ceb1d960303f3",
   "sequence_number":2
}
```

**Get Event (FTP RETR):**

```json
{
   "source_instance":"unFTP",
   "hostname":"MYMAC-XYZ",
   "payload":{
      "Get":{
         "path":"hello.txt"
      }
   },
   "username":"hannes",
   "trace_id":"0x687ee52555459a9c",
   "sequence_number":2
}
```

**Make Directory Event (FTP MKD):**

```json
{
   "source_instance":"unFTP",
   "hostname":"MYMAC-XYZ",
   "payload":{
      "MakeDir":{
         "path":"/x"
      }
   },
   "username":"hannes",
   "trace_id":"0x687ee52555459a9c",
   "sequence_number":3
}
```

**Rename Event (FTP RNFR and RNTO):**

```json
{
   "source_instance":"unFTP",
   "hostname":"MYMAC-XYZ",
   "payload":{
      "Rename":{
         "from":"/x",
         "to":"/y"
      }
   },
   "username":"hannes",
   "trace_id":"0x687ee52555459a9c",
   "sequence_number":4
}
```

**Put Event (FTP STOR):**

```json
{
   "source_instance":"unFTP",
   "hostname":"MYMAC-XYZ",
   "payload":{
      "Put":{
         "path":"x.yaml"
      }
   },
   "username":"hannes",
   "trace_id":"0x687ee52555459a9c",
   "sequence_number":5
}
```
