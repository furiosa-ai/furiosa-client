# Furiosa Web Service API Client
This is a client of Furiosa Web service API.

# Building

The library embeds the API endpoint depending on a specified cargo feature. 
By default, `cargo build` will embed the sandbox API endpoint for testing and development.

If you build with the feature 'local_api', it will embed the `http://localhost:8080/api/v1` for the API endpoint.
It will be useful for debugging.

To build the library and executable files using local API endpoint:
```
cargo build --features "local_api"
```

To build the library and executable files using production API endpoint:
```
cargo build --release --features "production_api"
```

Both require API keys. 
Please watch [this video](https://drive.google.com/file/d/1DLj4i6SEvGeq5eDnemTK15Trajamc8LW/view?usp=sharing) 
in order to learn how to generate API keys.

# How to Set API keys


## Shell environment variables
Please set the two environment variables as follow and then run your program:
```sh
export FURIOSA_ACCESS_KEY_ID=XXXXXXXXXXXXXXXXXXXXXXXXXXXXX
export FURIOSA_SECRET_ACCESS_KEY=YYYYYYYYYYYYYYYYYYYYYYYYYY
``` 

## Credential file
Please put your API keys at `$HOME/.furiosa/credential` as follow:
```
FURIOSA_ACCESS_KEY_ID=XXXXXXXXXXXXXXXXXXXXXXXXXXXXX
FURIOSA_SECRET_ACCESS_KEY=YYYYYYYYYYYYYYYYYYYYYYYYYY
```