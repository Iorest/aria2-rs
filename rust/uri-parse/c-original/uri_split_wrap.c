/* Test-only C-ABI wrapper around aria2 1.37.0 src/uri_split.c.
 * Packs the split result into one scalar so rust-anything can bridge it. */
#include "uri_split.h"
#include <stdint.h>

int64_t uri_split_summary(const char* uri)
{
  uri_split_result res;
  if (uri_split(&res, uri) != 0) {
    return -1;
  }
  uint64_t out = (uint64_t)(uint32_t)res.field_set;
  out |= (uint64_t)(uint32_t)res.port << 32;
  out |= (uint64_t)(uint32_t)res.flags << 48;
  return (int64_t)out;
}
