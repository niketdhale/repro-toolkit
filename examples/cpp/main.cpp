// Minimal C++ example using the RAII wrapper in repro_toolkit.hpp.
//
// Build and run via the provided Makefile: `make run`.
#include <iostream>

#include "repro_toolkit.hpp"

int main(int argc, char **argv) {
    if (argc < 2) {
        std::cerr << "usage: " << argv[0] << " <ecu.pdx> [custom-sequence.json]\n";
        return 1;
    }

    const std::string pdx_path = argv[1];

    auto default_sequence = repro_toolkit::parse_pdx(pdx_path);
    if (!default_sequence.valid()) {
        std::cerr << "pdx_path was empty/null\n";
        return 1;
    }
    std::cout << "=== default sequence ===\n" << default_sequence.str() << "\n\n";

    if (argc >= 3) {
        std::string sequence_path = argv[2];
        auto custom_sequence = repro_toolkit::generate_sequence(pdx_path, sequence_path);
        std::cout << "=== custom sequence ===\n" << custom_sequence.str() << "\n";
    }

    return 0;
}
