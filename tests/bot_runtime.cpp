// Native self-check: clang++ -std=c++17 -DESP32 -I app/tests/bot_stubs app/tests/bot_runtime.cpp -o app/target/bot-runtime.exe
#include <cassert>
#include <limits>
#include "../src/atelier_bot.h"
struct Mesh {
  unsigned sent = 0;
  std::string last;
  bool sendAtelierBot(uint32_t, const char* hashtag, const char* text) {
    assert(std::string(hashtag) == "#meteo"); ++sent; last = text; return true;
  }
};
int main() {
  using namespace AtelierBot;
  Mesh mesh;
  const uint32_t day = 20000 * 86400UL;
  begin();
  loop(mesh, day + 7 * 3600);
  assert(mesh.sent == 1);
  loop(mesh, day + 7 * 3600 + 20); assert(mesh.sent == 1);
  sensor.temperature = -5;
  loop(mesh, day + 8 * 3600);
  State state; flash.getBytes("weather", &state, sizeof(state));
  assert(state.low == -5 && state.high == 12 && state.recordLow == -5);
  begin(); // Reopen the same flash namespace, as after a restart.
  sensor.temperature = 25;
  loop(mesh, day + 20 * 3600);
  loop(mesh, day + 21 * 3600);
  assert(mesh.sent == 2 && mesh.last == "min -5.0 max 25.0");
  begin(); loop(mesh, day + 21 * 3600 + 5); assert(mesh.sent == 2);
  sensor.temperature = 10;
  loop(mesh, day + 86400);
  flash.getBytes("weather", &state, sizeof(state));
  assert(state.low == 10 && state.high == 10 && state.recordLow == -5 && state.recordHigh == 25);
  sensor.temperature = std::numeric_limits<float>::quiet_NaN();
  loop(mesh, day + 86400 + 7 * 3600); assert(mesh.sent == 2);
  sensor.temperature = 20;
  flash.failWrite = true;
  loop(mesh, day + 2 * 86400 + 7 * 3600); assert(mesh.sent == 2);
  flash.failWrite = false;
  loop(mesh, day - 86400 + 7 * 3600); assert(mesh.sent == 2);
}
