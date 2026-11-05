# Admission fixture: an aliased import.
package authz

import data.foo as bar

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
