# 🕵️ request-psn

[![MegaLinter](https://github.com/diz-unimr/request-psn/actions/workflows/mega-linter.yml/badge.svg)](https://github.com/diz-unimr/request-psn/actions/workflows/mega-linter.yml)
[![build](https://github.com/diz-unimr/request-psn/actions/workflows/build.yaml/badge.svg)](https://github.com/diz-unimr/request-psn/actions/workflows/build.yaml)
[![docker](https://github.com/diz-unimr/request-psn/actions/workflows/release.yaml/badge.svg)](https://github.com/diz-unimr/request-psn/actions/workflows/release.yaml)
[![codecov](https://codecov.io/gh/diz-unimr/request-psn/graph/badge.svg?token=Izcyq8RwyX)](https://codecov.io/gh/diz-unimr/request-psn)


> request-psn

This service provides pseudonymization of a single value and fixed target domain with gPAS.
The target domain is configured with the user's authorization data.

## API endpoints

The (REST) service is comprised of two endpoints:
One endpoint to pseudonymize a value with a pre-configured domain and a status endpoint to check the services health.

### <code>GET</code> <code><b>/request-psn/{original}</b></code> <code>(get pseudonym for {original})</code>

#### Responses

> | http code                    | content-type                | response                |
> |------------------------------|-----------------------------|-------------------------|
> | `200` Ok                     | `text/plain; charset=utf-8` | The pseudonymized value |
> | `401` Unauthorized           |                             |                         |
> | `500` Internal Server Error  | `text/plain;charset=UTF-8`  | Error message           |

### <code>GET</code> <code><b>/status</b></code> <code>(get service status information)</code>

#### Responses

> | http code          | content-type       | response                      |
> |--------------------|--------------------|-------------------------------|
> | `200` Ok           | `application/json` | `{"healthy": [true\|false] }` |
> | `401` Unauthorized | |                               |

### Authorization

Currently, only Basic authentication is supported. The credentials and the configured domain to
be used with requests are stored with the service.

## Configuration properties

Application properties are read from a properties file ([app.yaml](./app.yaml)) with default values.

| Name                | Default | Description                             | Required |
|---------------------|---------|-----------------------------------------|----------|
| `log_level`         | info    | Log level (error,warn,info,debug,trace) |          |
| `ttp.gpas.base_url` |         | gPAS base url                           | ✓        |
| `ttp.timeout`       | 120     | Retry timeout                           |          |

### Environment variables

Override configuration properties by providing environment variables with their respective property names. Replace `.`
with double underscore (`__`).

## Example deployment

Docker compose:

```yaml
query:
  image: ghcr.io/diz-unimr/request-psn:latest
  environment:
    LOG_LEVEL: debug
    TTP__GPAS__BASE_URL: http://localhost:8081
    TTP__TIMEOUT: 60
```

## License

[AGPL-3.0](https://www.gnu.org/licenses/agpl-3.0.en.html)
