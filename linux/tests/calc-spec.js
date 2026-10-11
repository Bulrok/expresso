// SPDX-License-Identifier: GPL-3.0-or-later
// Runs the shared spec (spec/cases.json) against the GJS evaluator.
//   gjs -m linux/tests/calc-spec.js
import GLib from 'gi://GLib';
import {findAndEvaluate} from '../extension/expresso@bulrok/calc.js';

const here = GLib.path_get_dirname(GLib.filename_from_uri(import.meta.url)[0]);
const specPath = GLib.build_filenamev([here, '..', '..', 'spec', 'cases.json']);
const [, bytes] = GLib.file_get_contents(specPath);
const {cases} = JSON.parse(new TextDecoder().decode(bytes));

let fail = 0;
for (const {input, result} of cases) {
    const got = findAndEvaluate(input)?.display ?? null;
    if (got !== result) {
        fail++;
        print(`FAIL ${JSON.stringify(input)} -> ${JSON.stringify(got)} (want ${JSON.stringify(result)})`);
    }
}

// Worst-case speed: a long line of words followed by a long expression.
const long = `${'word '.repeat(40)}${'1+'.repeat(100)}1`;
const n = 2000;
const t0 = GLib.get_monotonic_time();
for (let i = 0; i < n; i++)
    findAndEvaluate(long);
const us = (GLib.get_monotonic_time() - t0) / n;

print(`${cases.length - fail}/${cases.length} spec cases passed; long-line eval ${us.toFixed(1)} µs/call`);
if (fail)
    imports.system.exit(1);
