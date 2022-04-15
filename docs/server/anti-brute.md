---
title: Anti password guessing
---

To protect your FTP server against [password guessing attacks](https://en.wikipedia.org/wiki/Brute-force_attack), you can enable a "Failed logins policy".
This temporarily blocks the account if too many failed logins occur.

You can start up unFTP with the default failed logins policy:

```sh
unftp --failed-logins-policy
```

By default, 3 subsequent unsuccessful login attempts block further login attempts for that user orginating from that source IP during 5 minutes.

How to adjust these parameters and the default policy is explained in the sections below.

## Adjust max attempts and duration

To change this to block after `5` attempts instead of `3`:

```sh
unftp --failed-logins-policy --failed-max-attempts 5
```

To change this to block for 30 minutes (`1800` seconds) instead of 5 minutes (`300` seconds):

```sh
unftp --failed-logins-policy --failed-expire-after 1800
```

## Choose another policy

By default, the temporary block applies to the _combination of username and source IP_.
When an attacker is blocked by this policy, attempts to other accounts, or attempts from different source IPs can still continue.
You can adjust this behavior by blocking based on source IP or username instead.

Block by source IP instead of both IP and username:

```sh
unftp --failed-logins-policy ip
```

Block by username instead of both IP and username:

```sh
unftp --failed-logins-policy user
```

Be aware that user level -only blocking may also affect legitimate login attempts.
