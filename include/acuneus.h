#pragma once

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct CuneusInstance CuneusInstance;

typedef enum CuneusStatus {
    CUNEUS_STATUS_OK = 0,
    CUNEUS_STATUS_NULL = 1,
    CUNEUS_STATUS_INVALID_ARGUMENT = 2,
    CUNEUS_STATUS_NOT_FOUND = 3,
    CUNEUS_STATUS_IO_ERROR = 4,
} CuneusStatus;

typedef enum CuneusParamType {
    CUNEUS_PARAM_F32 = 0,
    CUNEUS_PARAM_COLOR3 = 1,
    CUNEUS_PARAM_ACTION = 2,
    CUNEUS_PARAM_STRING = 3,
    CUNEUS_PARAM_BOOL = 4,
    CUNEUS_PARAM_SELECT = 5,
} CuneusParamType;

typedef struct CuneusParamDesc {
    const char* id;
    const char* label;
    const char* group;
    CuneusParamType param_type;
    float min_value;
    float max_value;
    float default_value;
    uint32_t flags;
    const char* options;
} CuneusParamDesc;

typedef enum CuneusBinFlags {
    CUNEUS_BIN_USES_MOUSE = 1u << 0,
    CUNEUS_BIN_USES_KEYBOARD = 1u << 1,
} CuneusBinFlags;

typedef struct CuneusBinDesc {
    const char* name;
    const char* title;
    const char* source_file;
    uint32_t default_width;
    uint32_t default_height;
    uint32_t flags;
    const char* keys;
} CuneusBinDesc;

size_t cuneus_bin_count(void);
const char* cuneus_bin_name(size_t index);
CuneusStatus cuneus_bin_desc(size_t index, CuneusBinDesc* out_desc);
const char* cuneus_bin_title(const char* bin_name);
const char* cuneus_bin_keys(const char* bin_name);
bool cuneus_bin_uses_mouse(const char* bin_name);
bool cuneus_bin_uses_keyboard(const char* bin_name);
bool cuneus_bin_default_dimensions(const char* bin_name, uint32_t* out_width, uint32_t* out_height);

CuneusInstance* cuneus_instance_open(const char* bin_name, const char* executable_dir, uint16_t remote_port);
CuneusInstance* cuneus_instance_open_with_feedback(const char* bin_name, const char* executable_dir, uint16_t remote_port, uint16_t osc_feedback_port);
CuneusInstance* cuneus_instance_open_embedded(const char* bin_name, uint16_t remote_port);
CuneusInstance* cuneus_instance_open_embedded_with_feedback(const char* bin_name, uint16_t remote_port, uint16_t osc_feedback_port);
void cuneus_instance_free(CuneusInstance* instance);
CuneusStatus cuneus_instance_poll_child(CuneusInstance* instance, int32_t* out_exit_code);

const char* cuneus_last_error(void);

size_t cuneus_param_count(CuneusInstance* instance);
CuneusStatus cuneus_param_desc(CuneusInstance* instance, size_t index, CuneusParamDesc* out_desc);
CuneusStatus cuneus_set_param_f32(CuneusInstance* instance, const char* id, float value);
CuneusStatus cuneus_set_param_color3(CuneusInstance* instance, const char* id, float r, float g, float b);
CuneusStatus cuneus_set_param_string(CuneusInstance* instance, const char* id, const char* value);
CuneusStatus cuneus_set_param_bool(CuneusInstance* instance, const char* id, bool value);
CuneusStatus cuneus_trigger_action(CuneusInstance* instance, const char* id, float value);
CuneusStatus cuneus_load_media(CuneusInstance* instance, const char* path);
CuneusStatus cuneus_pulse(CuneusInstance* instance, float velocity);
CuneusStatus cuneus_note(CuneusInstance* instance, float pitch, float velocity);
CuneusStatus cuneus_set_transport(CuneusInstance* instance, float bpm, float beat, float measure);
CuneusStatus cuneus_set_overlay_visible(CuneusInstance* instance, bool visible);
CuneusStatus cuneus_toggle_overlay(CuneusInstance* instance);
CuneusStatus cuneus_set_window_title(CuneusInstance* instance, const char* title);
CuneusStatus cuneus_set_window_title_bar_visible(CuneusInstance* instance, bool visible);
CuneusStatus cuneus_hide_window_title_bar(CuneusInstance* instance);
CuneusStatus cuneus_set_window_position(CuneusInstance* instance, int32_t x, int32_t y);
CuneusStatus cuneus_get_window_position(CuneusInstance* instance, int32_t* out_x, int32_t* out_y);
CuneusStatus cuneus_set_window_scale(CuneusInstance* instance, float scale);
CuneusStatus cuneus_set_window_size(CuneusInstance* instance, uint32_t width, uint32_t height);
CuneusStatus cuneus_set_time(CuneusInstance* instance, float time_seconds);
CuneusStatus cuneus_set_fps(CuneusInstance* instance, float fps);
CuneusStatus cuneus_set_resolution(CuneusInstance* instance, uint32_t width, uint32_t height);
CuneusStatus cuneus_set_audio_spectrum(CuneusInstance* instance, const float* values, size_t count);
CuneusStatus cuneus_discover(CuneusInstance* instance);
CuneusStatus cuneus_subscribe(CuneusInstance* instance, bool enabled);

#ifdef __cplusplus
}
#endif
