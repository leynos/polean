# Admission fixture: the owns-resource atom alone.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.id == input.resource.owner
}
