// A small RAII wrapper over repro-toolkit-ffi's C ABI, so callers never
// have to remember to call repro_toolkit_free_string themselves.
#pragma once

#include <memory>
#include <optional>
#include <string>

extern "C" {
char *repro_toolkit_parse_pdx(const char *path);
char *repro_toolkit_generate_sequence(const char *pdx_path, const char *sequence_path);
void repro_toolkit_free_string(char *ptr);
}

namespace repro_toolkit {

// Owns a string returned by the FFI layer and frees it correctly on
// destruction (via repro_toolkit_free_string, not delete/free).
class OwnedJson {
public:
    explicit OwnedJson(char *raw) : raw_(raw) {}

    OwnedJson(const OwnedJson &) = delete;
    OwnedJson &operator=(const OwnedJson &) = delete;

    OwnedJson(OwnedJson &&other) noexcept : raw_(other.raw_) { other.raw_ = nullptr; }
    OwnedJson &operator=(OwnedJson &&other) noexcept {
        if (this != &other) {
            reset();
            raw_ = other.raw_;
            other.raw_ = nullptr;
        }
        return *this;
    }

    ~OwnedJson() { reset(); }

    bool valid() const { return raw_ != nullptr; }
    std::string str() const { return raw_ != nullptr ? std::string(raw_) : std::string(); }

private:
    void reset() {
        if (raw_ != nullptr) {
            repro_toolkit_free_string(raw_);
            raw_ = nullptr;
        }
    }

    char *raw_;
};

// Auto-generates the repro sequence from a PDX file.
inline OwnedJson parse_pdx(const std::string &pdx_path) {
    return OwnedJson(repro_toolkit_parse_pdx(pdx_path.c_str()));
}

// Generates the repro sequence, using a custom sequence file when given.
inline OwnedJson generate_sequence(const std::string &pdx_path,
                                    const std::optional<std::string> &sequence_path) {
    const char *seq_ptr = sequence_path.has_value() ? sequence_path->c_str() : nullptr;
    return OwnedJson(repro_toolkit_generate_sequence(pdx_path.c_str(), seq_ptr));
}

} // namespace repro_toolkit
