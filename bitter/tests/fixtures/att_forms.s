# One instruction per AT&T operand form, read by bitter/tests/att.rs both as
# GNU `as` source and, under `pub _start:`, as BitterASM.
mov %rbx, %rax
mov %ebx, %eax
mov %bx, %ax
mov %bl, %al
mov %r13d, %r14d
mov $5, %rax
mov $5, %eax
mov $5, %al
mov (%rbx), %rax
mov 8(%rbx), %eax
mov -4(%rbp), %ecx
mov (%rbx,%rcx), %rdx
mov 16(%rbx,%rcx), %rdx
mov (%rbx,%rcx,4), %rdx
mov 0x10(%rbx,%rcx,8), %rdx
mov %rax, (%rbx)
mov %eax, 8(%rbx)
mov %dl, 1(%rdi)
mov %al, 68(%r9,%rbx)
movb $124, 67(%r9)
movw $1, (%rax)
movl $2, 4(%rsp)
movq $-1, (%rdi)
add %rbx, %rax
add $3, %rdi
sub %r14, %rdx
and $15, %edx
or %ecx, %eax
xor %esi, %esi
xor %r13d, %r13d
cmp $-4, %rax
cmp $32, %eax
cmp %r14, %rbx
cmpq $2, (%rsp)
cmpb $0, (%rsi)
test %rax, %rax
test $1, %al
adc %rbx, %rax
sbb $1, %ecx
addl %eax, %ebx
addq $8, %rsp
subl $1, 8(%rbp)
xorl %eax, %eax
inc %rbx
dec %ecx
incq (%rdi)
decb 3(%rax)
neg %rax
not %edx
mul %rcx
div %ebx
idiv %r8
shl $4, %rax
shr $4, %edx
sar %cl, %rax
rol $1, %al
shlq $2, (%rdi)
shrl %cl, 4(%rsp)
movzbl (%r8,%rdx), %edx
movzbl %al, %eax
movzbq %dl, %rcx
movzwl (%rax), %ecx
movsbl %al, %ecx
movswq 2(%rbx), %rax
movslq %eax, %rcx
movslq 8(%rsp), %rdx
lea 15(%r9), %rdi
lea (%rbx,%rcx,8), %rax
leal 4(%rax), %ecx
imul %rcx
imul %rbx, %rax
imul 8(%rsp), %eax
imul $10, %rbx, %rax
push %rax
push $7
pushq 8(%rsp)
pop %rbx
popq (%rsp)
jmp *%rax
call *8(%rsp)
cld
rep stosb
rep movsq
repne scasb
stosl
syscall
ret
