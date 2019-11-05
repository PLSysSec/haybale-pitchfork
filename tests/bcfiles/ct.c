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

// not constant-time due to memory access on one path
int notct_onepath(int x, int y) {
  volatile int z[3] = { 0, 2, 300 };
  z[2] = y;
  if (z[2] > 3) {
    return z[x % 3];
  } else {
    return z[1];
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
