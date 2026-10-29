# Admission fixture: a different import.
package authz

import data.foo

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
