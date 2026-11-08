#pragma once
#include <cstring>
#include <vector>
struct Preferences {
  std::vector<unsigned char> bytes;
  bool failWrite = false;
  bool begin(const char*, bool) { return true; }
  size_t getBytesLength(const char*) { return bytes.size(); }
  size_t getBytes(const char*, void* dest, size_t len) {
    if (len != bytes.size()) return 0;
    memcpy(dest, bytes.data(), len); return len;
  }
  size_t putBytes(const char*, const void* src, size_t len) {
    if (failWrite) return 0;
    bytes.assign((const unsigned char*)src, (const unsigned char*)src + len); return len;
  }
};
