# Admission fixture: a single-literal body on one line.
package authz

import rego.v1

default allow := false

allow if { input.subject.tenant == input.resource.tenant }
