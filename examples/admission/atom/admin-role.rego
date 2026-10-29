# Admission fixture: the admin-role atom alone.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.role == "admin"
}
