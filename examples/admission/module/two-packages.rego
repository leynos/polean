# Admission fixture: a second package declaration.
package authz

package other

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
