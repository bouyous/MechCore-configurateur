#pragma once
#include <cstdint>
#include <cstdio>
#include <string>
class String {
  std::string value;
public:
  String(const char* s): value(s) {}
  String(float f, int) { char b[32]; snprintf(b, sizeof(b), "%.1f", f); value = b; }
  void replace(const char* key, const String& replacement) {
    size_t pos = 0;
    while ((pos = value.find(key, pos)) != std::string::npos) {
      value.replace(pos, std::string(key).size(), replacement.value); pos += replacement.value.size();
    }
  }
  size_t length() const { return value.size(); }
  const char* c_str() const { return value.c_str(); }
};
struct TwoWire { TwoWire(int) {} bool begin(int, int) { return true; } };
struct Adafruit_BME280 {
  float temperature = 12;
  bool begin(int, TwoWire*) { return true; }
  float readTemperature() { return temperature; }
  float readPressure() { return 101320; }
};
inline struct SerialStub { void println(const char*) {} } Serial;
