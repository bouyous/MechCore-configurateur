#pragma once
namespace AtelierBot {
constexpr int SDA_PIN=4, SCL_PIN=5, ADDRESS=0x76, UTC_OFFSET=0;
static const char* hashtags[] = {"#meteo"};
struct Message { bool enabled; unsigned minute; unsigned channel; const char* text; };
static const Message messages[] = {{true, 420, 0, "{temperature}"}, {true, 1260, 0, "min {min} max {max}"}};
}
