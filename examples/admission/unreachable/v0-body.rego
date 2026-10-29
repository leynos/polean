# Admission fixture: a Rego v0 rule body.
package authz

import rego.v1

default allow := false

allow {
    input.subject.tenant == input.resource.tenant
}
