# Unix SMB/CIFS implementation.
#
# Tests for Rust
#
# Copyright (C) David Mulder <dmulder@samba.org> 2024
#
# This program is free software; you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation; either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.
#

"""Cargo tests for Rust sources"""

from samba.tests import TestCase, BlackboxProcessError
import os
from subprocess import Popen, PIPE
from samba import is_rust_built


class RustCargoTests(TestCase):
    def setUp(self):
        super().setUp()

        # Locate the rust source directory
        self.rust_dir = os.path.abspath(
                os.path.join(
                    os.path.realpath(
                        os.path.dirname(__file__)
                    ),
                    '../../../../rust'
                )
            )

        # Locate the bin directory
        self.target_dir = os.path.abspath(
                os.path.join(
                    os.path.realpath(
                        os.path.dirname(__file__)
                    ),
                    '../../..',
                    'default/rust',
                )
            )

    def check_cargo_test(self, crate_toml):
        # Execute the cargo test command
        cmd = 'cargo test --target-dir=%s --manifest-path=%s' % (self.target_dir, crate_toml)
        p = Popen(cmd, stdout=PIPE, stderr=PIPE, shell=True)
        stdoutdata, stderrdata = p.communicate()
        retcode = p.returncode
        if retcode != 0:
            msg = "cargo test failed; return code %s" % retcode
            raise BlackboxProcessError(retcode,
                                       cmd,
                                       stdoutdata.decode('utf-8'),
                                       stderrdata.decode('utf-8'),
                                       msg)

    def test_rust(self):
        if not is_rust_built():
            self.skipTest('Cannot test Samba Rust if not built')

        crates = []
        for root, dirs, files in os.walk(self.rust_dir):
            for file in files:
                if os.path.basename(file) == 'Cargo.toml':
                    if root != self.rust_dir:
                        crates.append(os.path.join(root, file))

        for crate_toml in crates:
            with self.subTest(crate_toml):
                self.check_cargo_test(crate_toml)
