"""Make the provisioning script importable from the test package.

The script lives beside this directory rather than in an installed package, so
pytest's default import mode would not find it. Inserting the script directory
on `sys.path` keeps the tests exercising the same module that
`make tools` runs.
"""

from __future__ import annotations

import sys
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]

if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))
