# Admission fixture: two imports, where the profile admits one.
package authz

import data.alpha
import data.beta

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
