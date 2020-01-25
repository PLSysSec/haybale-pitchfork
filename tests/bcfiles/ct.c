// simple constant-time function
int ct_simple(int x) {
  return x + 3;
}

// still constant-time, despite having a conditional and memory accesses
int ct_simple2(int x, int y) {
  volatile int z = 2;
  if (z > 3) {
    return x * 5;
  } else {
    return y / 99;
  }
}

// not constant-time due to branching
int notct_branch(int x) {
  if (x > 10) {
    return x % 200 * 3;
  } else {
    return x + 10;
  }
}

// not constant-time due to memory access
int notct_mem(int x) {
  volatile int z[3] = { 0, 2, 300 };
  return z[x % 3];
}

// not constant-time due to memory access on the "true" path; but no violation
// on the "else" path
int notct_truepath(int x, int y, int notsecret) {
  volatile int z[3] = { 0, 2, 300 };
  z[2] = y;
  if (notsecret > 3) {
    return z[x % 3];  // address depends on x, which is a violation
  } else {
    return z[1];
  }
}

// not constant-time due to memory access on the "else" path; but no violation
// on the "true" path
int notct_falsepath(int x, int y, int notsecret) {
  volatile int z[3] = { 0, 2, 300 };
  z[2] = y;
  if (notsecret > 3) {
    return z[1];
  } else {
    return z[x % 3];  // address depends on x, which is a violation
  }
}

// constant-time violations on two different paths
// (although no violation on the third)
int two_ct_violations(int x, int y, int notsecret) {
  volatile int z[3] = { 0, 2, 300 };
  z[2] = y;
  if (notsecret < 3) {
    return z[x % 3];  // address depends on x, which is a violation
  } else if (notsecret > 100) {
    return z[0];
  } else {
    return z[y - 2];  // address depends on y, which is a violation
  }
}

// constant-time in one argument but not the other
int ct_onearg(int x, int y) {
  if (x > 100) {
    return y;
  } else {
    return x % 20 * 3;
  }
}

// constant-time in secrets
int ct_secrets(int* secretarr) {
  return secretarr[20] + 3;
}

// not constant-time in secrets
int notct_secrets(int* secretarr) {
  if (secretarr[20] > 3) {
    return secretarr[0] * 3;
  } else {
    return secretarr[2] / 22;
  }
}

struct PartiallySecret {
  int notsecret;
  int secret;
};

// constant-time in the secret
int ct_struct(int* publicarr, struct PartiallySecret* ps) {
  return publicarr[ps->notsecret] + ps->secret;
}

// not constant-time in the secret
int notct_struct(int* publicarr, struct PartiallySecret* ps) {
  return publicarr[ps->secret] + ps->notsecret;
}

// not constant-time, on the path where `maybenull` is NULL
int notct_maybenull_null(int* publicarr, int* maybenull, struct PartiallySecret* ps) {
  if (!maybenull) {
    return publicarr[ps->secret];
  } else {
    return publicarr[ps->notsecret];
  }
}

// not constant-time, on the path where `maybenull` is not NULL
int notct_maybenull_notnull(int* publicarr, int* maybenull, struct PartiallySecret* ps) {
  if (maybenull) {
    return maybenull[ps->secret];
  } else {
    return publicarr[ps->notsecret];
  }
}

// pointer to pointer to secret
int ct_doubleptr(int** secretarrs) {
  return secretarrs[2][5] + 3;
}

int notct_doubleptr(int** secretarrs) {
  if (secretarrs[2][5] > 3) {
    return secretarrs[0][10] * 3;
  } else {
    return secretarrs[2][22] / 5;
  }
}

// void pointer, casted to struct pointer, constant-time
int ct_struct_voidptr(int* publicarr, void* voidptr) {
  struct PartiallySecret* ps = (struct PartiallySecret*) voidptr;
  return publicarr[ps->notsecret] + ps->secret;
}

// void pointer, casted to struct pointer, not constant-time
int notct_struct_voidptr(int* publicarr, void* voidptr) {
  struct PartiallySecret* ps = (struct PartiallySecret*) voidptr;
  return publicarr[ps->secret] + ps->notsecret;
}

struct Child;  // forward declaration

struct Parent {
  int x;
  struct Child* child1;
  struct Child* child2;
};

struct Child {
  int y;
  struct Parent* parent;
};

int indirectly_recursive_struct(int* publicarr, struct Parent* parent) {
  return publicarr[parent->child2->parent->x];
}

// x and length are public, this function is constant-time if x <= length and not if x > length
int related_args(unsigned length, unsigned x, int secret) {
  int arr[20];  // first `length` bytes are public, rest are secret
  for (unsigned i = length; i < 20; i++) {
    arr[i] = secret;
  }
  if (arr[x]) {
    return arr[0] * 33 + length + x;
  } else {
    return 1;
  }
}

struct StructWithRelatedFields {
  unsigned length;
  unsigned x;
  int secret;
};

int struct_related_fields(struct StructWithRelatedFields* s) {
  int arr[20];  // first `length` bytes are public, rest are secret
  for (unsigned i = s->length; i < 20; i++) {
    arr[i] = s->secret;
  }
  if (arr[s->x]) {
    return arr[0] * 33 + s->length + s->x;
  } else {
    return 1;
  }
}
