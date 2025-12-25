---
title: User config over HTTP
---

This page explains how to serve user configuration over HTTP instead of using local JSON files, allowing for dynamic user management without restarting the server.

In the "Per User Config" section we showed how to create configuration unique for each user.
This
configuration was stored in a JSON file. This same JSON file can be consumed by unFTP over HTTP.

Start be defining the user configuration:

```shell
touch user-options.json
```

```json
[
  {
    "username": "alice",
    "vfs_perms": [
      "-mkdir",
      "-rmdir",
      "-del",
      "-ren",
      "-md5"
    ],
    "root": "alice",
    "account_enabled": true
  },
  {
    "username": "bob",
    "vfs_perms": [
      "none",
      "+put",
      "+list",
      "+md5"
    ],
    "root": "bob"
  }
]
```

Then serve this over HTTP with any method you prefer. Here is an example doing with a Go script:

```shell
touch main.go
```

```go
package main

import (
	"fmt"
	"net/http"
	"os"
)

func main() {
	// Specify the path to the JSON file containing user details
	jsonFilePath := "./user-options.json"

	// Create a simple HTTP handler function
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {

    fmt.Println("Requested URL: ", r.URL)

		// Check if the request method is GET
		if r.Method == http.MethodGet {
			// Read the contents of the JSON file
			jsonData, err := os.ReadFile(jsonFilePath)
			if err != nil {
				http.Error(w, fmt.Sprintf("Error reading JSON file: %s", err), http.StatusInternalServerError)
				return
			}

			// Set the Content-Type header to indicate JSON content
			w.Header().Set("Content-Type", "application/json")

			// Write the JSON data to the response writer
			w.Write(jsonData)
		} else {
			// If the request method is not GET, respond with a 405 Method Not Allowed status
			http.Error(w, "Method Not Allowed", http.StatusMethodNotAllowed)
		}
	})

	// Start the web server on port 8080
	fmt.Println("Server is running on http://localhost:8080")
	err := http.ListenAndServe(":8080", nil)
	if err != nil {
		fmt.Printf("Error starting server: %s\n", err)
	}
}
```

You'll need [Go](https://go.dev/) installed and then you can do:

```go
go run main.go
```

and then notice the HTTP address in the output:

```
Server is running on http://localhost:8080
```

You then run unFTP with the `--usr-http-url` command line argument:

```shell
unftp \
    --root-dir=. \
    --auth-type=json \
    --auth-json-path=credentials.json \
    --usr-http-url='http://localhost:8080/users/'
```

Notice that when running unFTP, we still do `--auth-type=json` and `--auth-json-path=credentials.json`. The
authenticator and user detail provider mechanisms are disjoint. So passwords are specified in `credentials.json` and
the user details or options like where the user's root path is gets served over HTTP.

Create the credentials file:

```shell
touch credentials.json
```

and add this as contents:

```json
[
  {
    "username": "alice",
    "password": "12345678"
  },
  {
    "username": "bob",
    "password": "secret"
  }
]
```

Let us also make home directories for Bob and Alice:

```shell
mkdir bob
mkdir alice
touch bob/hello-bob.txt
touch alice/hello-alice.txt
```

Finally, let us run an FTP client

```shell
lftp localhost -p 2121 -u bob
```

Provide the password 'secret' for Bob do an FPT `ls`.

You should see the file `hello-bob.txt` being listed and the output of the little Go webserver should be:

```
Requested URL:  /users/bob
```

Now that we've covered user configuration over HTTP, you may want to explore [Pub/Sub event notifications](/server/pubsub) or configure [anti-brute force protection](/server/anti-brute).
