---
title: Enabling TLS
---


Start by generating a self signed certificate

```sh
openssl req \
   -x509 \
   -newkey rsa:2048 \
   -nodes \
   -keyout unftp.key \
   -out unftp.crt \
   -days 3650 \
   -subj '/CN=www.myunftp.domain/O=My Company Name LTD./C=NL'
```

The run unFTP, pointing it to the certificate and key. You can use the `--ftps-required-on-control-channel` setting to enforce TLS on the FTP control channel. In other words an FTP client will only be allowed to use FTP commands if it upgrades to a private TLS connection.

```sh
./unftp \
  --root-dir=/home/unftp/data \
  --ftps-certs-file=/home/unftp/unftp.crt \
  --ftps-key-file=/home/unftp/unftp.key \
  --ftps-required-on-control-channel=all

```

## Setting up Mutual TLS

Create Server Root Key and Certificate:

```sh
openssl genrsa -out unftp_client_ca.key 2048
openssl req -new -x509 -days 365 \
	-key unftp_client_ca.key \
        -subj '/CN=unftp-ca.mysite.com/O=bol.com/C=NL' \
	-out unftp_client_ca.crt
````

Create a client side key:

```
openssl genrsa -out client.key 2048
```

Create a client side certificate signing request (CSR):

```
openssl req -new -sha256 \
    -key client.key \
    -subj '/CN=unftp-client.mysite.com/O=bol.com/C=NL' \
    -reqexts SAN \
    -config <(cat /etc/ssl/openssl.cnf \
        <(printf "\n[SAN]\nsubjectAltName=DNS:localhost")) \
    -out client.csr
```

Sign the certificate with our own CA

```
openssl x509 -req \
  -in client.csr \
  -CA unftp_client_ca.crt \
  -CAkey unftp_client_ca.key \
  -CAcreateserial \
  -extfile <(printf "subjectAltName=DNS:localhost") \
  -out client.crt \
  -days 1024 \
  -sha256
```

Run unFTP pointing to the CA cert:

```
unftp \
  --root-dir=/home/unftp/data \
  --ftps-certs-file=/home/unftp/unftp.crt \
  --ftps-key-file=/home/unftp/unftp.key \
  --ftps-required-on-control-channel=all \
  --ftps-client-auth=require \
  --ftps-trust-store=/Users/xxx/unftp/unftp_client_ca.crt
```

From another terminal: Connect with CURL, sending the client certificate:

```
curl -v \
  --insecure \
  --user 'test:test' \
  --ftp-ssl --ssl-reqd \
  --ftp-pasv --disable-epsv \
  --cacert unftp_client_ca.crt \
  --cert client.crt \
  --key client.key \
  --cert-type PEM \
  --pass '' \
  --tlsv1.2 \
  ftp://localhost:2121/  
```
