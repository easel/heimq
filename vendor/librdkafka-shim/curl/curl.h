/*
 * Stub for <curl/curl.h>, satisfying a stray include in librdkafka 2.12.1.
 *
 * librdkafka guards every OAUTHBEARER/OIDC *use* of libcurl with
 * `#if WITH_OAUTHBEARER_OIDC`, but guards the include in src/rdkafka_conf.c
 * with `#ifdef`. Since config.h is generated from `#cmakedefine01`, the macro
 * is always defined -- as 0 when OIDC is off -- so `#ifdef` is true and the
 * header is pulled in even though nothing here is ever referenced. The curl
 * symbols live in rdhttp.c and rdkafka_sasl_oauthbearer_oidc.c, which cmake
 * excludes from the build entirely when OIDC is off.
 *
 * rdkafka-sys builds librdkafka with -DWITH_CURL=0 -DWITH_SSL=0, so OIDC is
 * off and this file only has to parse. Vendoring it keeps the build hermetic
 * instead of requiring libcurl dev headers on every build host.
 *
 * This shim is reachable only via CPATH, set in .cargo/config.toml. If we ever
 * enable the rdkafka `curl` or `ssl` features, OIDC turns on, libcurl becomes a
 * genuine dependency, and this stub must be deleted -- it would otherwise
 * shadow the real header (CPATH is searched before the system include dirs).
 *
 * Upstream: the include should be `#if WITH_OAUTHBEARER_OIDC`.
 */

#ifndef HEIMQ_LIBRDKAFKA_CURL_SHIM_H
#define HEIMQ_LIBRDKAFKA_CURL_SHIM_H

#endif /* HEIMQ_LIBRDKAFKA_CURL_SHIM_H */
