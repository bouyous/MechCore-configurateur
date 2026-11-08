#pragma once
#if !defined(ESP32)
#error Atelier_weather_bot_requires_ESP32
#else
#include <Adafruit_BME280.h>
#include <Preferences.h>
#include <driver/gpio.h>
#include <soc/soc_caps.h>
#include <math.h>
#include "AtelierBotConfig.h"

namespace AtelierBot {
static_assert(SOC_I2C_NUM > 1, "Atelier_BME280_requires_ESP32_with_two_I2C_controllers");
static TwoWire bus(1);
static Adafruit_BME280 sensor;
static Preferences flash;
static bool ready = false;
struct State {
  uint32_t day;
  float low, high, recordLow, recordHigh;
  uint32_t sent[8];
};
static void begin() {
  static_assert(GPIO_IS_VALID_OUTPUT_GPIO(SDA_PIN), "SDA incompatible");
  static_assert(GPIO_IS_VALID_OUTPUT_GPIO(SCL_PIN), "SCL incompatible");
  char key[16];
  snprintf(key, sizeof(key), "am%u_%u_%u", SDA_PIN, SCL_PIN, ADDRESS);
  ready = flash.begin(key, false) && bus.begin(SDA_PIN, SCL_PIN) && sensor.begin(ADDRESS, &bus);
  if (!ready) Serial.println("Bot: BME280 ou flash indisponible, rapports suspendus.");
}
static String render(const char* text, float temperature, float pressure, const State& state) {
  String result(text);
  result.replace("{temperature}", String(temperature, 1));
  result.replace("{pression}", String(pressure, 1));
  result.replace("{min}", String(state.low, 1));
  result.replace("{max}", String(state.high, 1));
  result.replace("{record_min}", String(state.recordLow, 1));
  result.replace("{record_max}", String(state.recordHigh, 1));
  return result;
}
template<class Mesh> static void loop(Mesh& mesh, uint32_t now) {
  static uint32_t checked = 0;
  if (!ready || now < 1704067200UL || checked == now / 60) return;
  checked = now / 60;
  float temperature = sensor.readTemperature();
  float pressure = sensor.readPressure() / 100.0f;
  if (!isfinite(temperature) || temperature < -40 || temperature > 85 || !isfinite(pressure) || pressure < 300 || pressure > 1100) {
    Serial.println("Bot: mesure invalide, rapport suspendu."); return;
  }
  uint32_t local = now + UTC_OFFSET * 60;
  uint32_t day = local / 86400;
  State state = {};
  size_t size = flash.getBytesLength("weather");
  bool loaded = size == sizeof(state) && flash.getBytes("weather", &state, sizeof(state)) == sizeof(state);
  if (size && (!loaded || !isfinite(state.low) || !isfinite(state.high) || !isfinite(state.recordLow) || !isfinite(state.recordHigh)
      || state.low > state.high || state.recordLow > state.low || state.recordHigh < state.high)) {
    Serial.println("Bot: donnees flash invalides, rapport suspendu."); return;
  }
  bool changed = false;
  if (!loaded) {
    state.day = day; state.low = state.high = state.recordLow = state.recordHigh = temperature; changed = true;
  }
  if (day < state.day) { Serial.println("Bot: horloge en recul, attente de synchronisation."); return; }
  if (state.day != day) { state.day = day; state.low = state.high = temperature; changed = true; }
  if (temperature < state.low) { state.low = temperature; changed = true; }
  if (temperature > state.high) { state.high = temperature; changed = true; }
  if (temperature < state.recordLow) { state.recordLow = temperature; changed = true; }
  if (temperature > state.recordHigh) { state.recordHigh = temperature; changed = true; }
  // Commit each new extremum before sending; RAM is only a working copy of the flash record.
  if (changed && flash.putBytes("weather", &state, sizeof(state)) != sizeof(state)) {
    Serial.println("Bot: echec sauvegarde flash, rapport suspendu."); return;
  }
  for (unsigned i = 0; i < sizeof(messages) / sizeof(messages[0]); ++i) {
    const auto& message = messages[i];
    // Only the scheduled minute is sent: a reboot never floods old reports.
    if (!message.enabled || (local % 86400) / 60 != message.minute || state.sent[i] >= day) continue;
    String text = render(message.text, temperature, pressure, state);
    if (text.length() > 120) { Serial.println("Bot: message trop long."); continue; }
    if (mesh.sendAtelierBot(now, hashtags[message.channel], text.c_str())) {
      state.sent[i] = day;
      if (flash.putBytes("weather", &state, sizeof(state)) != sizeof(state)) {
        Serial.println("Bot: echec sauvegarde envoi, bot suspendu."); ready = false; return;
      }
    }
  }
}
}
#endif
