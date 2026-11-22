"""Enable a private stdin/stdout worker mode on the pinned Whisper CLI."""
import pathlib


def add_worker(cli: pathlib.Path):
    source = cli.read_text(encoding="utf-8")
    flag = "--localscribe-worker"
    if flag in source:
        return
    replacements = [
        ("    bool no_prints       = false;",
         "    bool localscribe_worker = false;\n    bool no_prints       = false;"),
        ('        else if (arg == "-np"',
         f'        else if (arg == "{flag}") {{ params.localscribe_worker = true; }}\n'
         '        else if (arg == "-np"'),
        ("    // number of input files that could not be processed",
         """    if (params.localscribe_worker) {
        if (params.fname_inp.size() != 1 || params.fname_inp[0] == "-" ||
            params.fname_out.size() != 1 || params.fname_out[0] == "-" || !params.output_jsn) {
            return 1;
        }
        params.no_prints = true;
        params.print_progress = false;
    }

    // number of input files that could not be processed"""),
        ("    for (int f = 0; f < (int) params.fname_inp.size(); ++f) {",
         """    for (int f = 0; f < (int) params.fname_inp.size(); f += params.localscribe_worker ? 0 : 1) {
        if (params.localscribe_worker && std::getchar() != 'R') {
            break;
        }"""),
        ("            print_segment_callback{is_stdout ? nullptr : whisper_print_segment_callback}",
         "            print_segment_callback{(is_stdout || params.localscribe_worker) ? nullptr : whisper_print_segment_callback}"),
        ('            fprintf(stderr, "error: failed to read audio file',
         '            if (params.localscribe_worker) {\n                n_failed++;\n                break;\n            }\n'
         '            fprintf(stderr, "error: failed to read audio file'),
        ("        if (fout_factory.failed) {\n            n_failed++;\n        }",
         """        if (fout_factory.failed) {
            n_failed++;
        }
        if (params.localscribe_worker) {
            fout_factory.fout.close();
            std::putchar(!n_failed && fout_factory.fout ? 'R' : 'E');
            std::fflush(stdout);
        }"""),
    ]
    for before, after in replacements:
        if source.count(before) != 1:
            raise RuntimeError(f"Cannot apply worker protocol to the pinned CLI near {before!r}")
        source = source.replace(before, after, 1)
    cli.write_text(source, encoding="utf-8")
