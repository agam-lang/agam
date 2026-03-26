; ModuleID = 'bench_advanced_fair_llvm.ll'
source_filename = "bench_advanced_fair_llvm.ll"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

@agam_argc_storage = internal unnamed_addr global i32 0
@agam_argv_storage = internal unnamed_addr global ptr null
@.str.0 = private unnamed_addr constant [35 x i8] c"Benchmark 1: Stateful integer loop\00"
@.str.2 = private unnamed_addr constant [6 x i8] c"%lld\0A\00"
@.str.3 = private unnamed_addr constant [26 x i8] c"Benchmark 2: Fibonacci(N)\00"
@.str.4 = private unnamed_addr constant [30 x i8] c"Benchmark 3: Count primes < N\00"
@.str.5 = private unnamed_addr constant [33 x i8] c"Benchmark 4: Matrix multiply NxN\00"
@.str.6 = private unnamed_addr constant [37 x i8] c"Benchmark 5: Polynomial accumulation\00"

; Function Attrs: nofree nounwind
declare noundef i32 @printf(ptr nocapture noundef readonly, ...) local_unnamed_addr #0

; Function Attrs: mustprogress nofree nounwind willreturn
declare noundef i64 @strtoll(ptr nocapture noundef readonly, ptr nocapture, i32 noundef) local_unnamed_addr #1

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: none, inaccessiblemem: none)
define noundef i32 @agam_argc() local_unnamed_addr #2 {
entry:
  %0 = load i32, ptr @agam_argc_storage, align 4
  ret i32 %0
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, inaccessiblemem: none)
define noundef ptr @agam_argv(i32 noundef %index) local_unnamed_addr #3 {
entry:
  %0 = load ptr, ptr @agam_argv_storage, align 8
  %1 = sext i32 %index to i64
  %2 = getelementptr inbounds ptr, ptr %0, i64 %1
  %3 = load ptr, ptr %2, align 8
  ret ptr %3
}

; Function Attrs: mustprogress nofree nounwind willreturn
define noundef i32 @agam_parse_int(ptr nocapture noundef readonly %s) local_unnamed_addr #1 {
entry:
  %0 = tail call i64 @strtoll(ptr nocapture noundef %s, ptr null, i32 noundef 10)
  %1 = trunc i64 %0 to i32
  ret i32 %1
}

; Function Attrs: nofree norecurse nosync nounwind memory(none)
define noundef i64 @agam_sum_loop(i64 noundef %p0) local_unnamed_addr #4 {
block_0:
  %v167 = icmp sgt i64 %p0, 0
  br i1 %v167, label %block_2.preheader, label %block_3

block_2.preheader:                                ; preds = %block_0
  %v711 = urem i64 %p0, 7919
  %v9 = add nuw nsw i64 %v711, 1
  br label %block_2

block_2:                                          ; preds = %block_2.preheader, %block_2
  %local_i.010 = phi i64 [ %v37, %block_2 ], [ 0, %block_2.preheader ]
  %local_state.09 = phi i64 [ %v27, %block_2 ], [ %v9, %block_2.preheader ]
  %local_total.08 = phi i64 [ %v33, %block_2 ], [ 0, %block_2.preheader ]
  %v19 = mul nsw i64 %local_state.09, 57
  %v22 = mul i64 %local_i.010, 13
  %v23 = add nsw i64 %v19, 17
  %v25 = add i64 %v23, %v22
  %v27 = srem i64 %v25, 1000003
  %v32.lhs.trunc = trunc i64 %v27 to i32
  %v326 = srem i32 %v32.lhs.trunc, 1024
  %v32.sext = sext i32 %v326 to i64
  %v33 = add i64 %local_total.08, %v32.sext
  %v37 = add nuw nsw i64 %local_i.010, 1
  %exitcond.not = icmp eq i64 %v37, %p0
  br i1 %exitcond.not, label %block_3, label %block_2

block_3:                                          ; preds = %block_2, %block_0
  %local_total.0.lcssa = phi i64 [ 0, %block_0 ], [ %v33, %block_2 ]
  ret i64 %local_total.0.lcssa
}

; Function Attrs: nofree norecurse nosync nounwind memory(none)
define noundef i64 @agam_fibonacci(i64 noundef %p0) local_unnamed_addr #4 {
block_5:
  %v534 = icmp sgt i64 %p0, 0
  br i1 %v534, label %block_7.preheader, label %block_8

block_7.preheader:                                ; preds = %block_5
  %xtraiter = and i64 %p0, 7
  %0 = icmp ult i64 %p0, 8
  br i1 %0, label %block_8.loopexit.unr-lcssa, label %block_7.preheader.new

block_7.preheader.new:                            ; preds = %block_7.preheader
  %unroll_iter = and i64 %p0, 9223372036854775800
  br label %block_7

block_7:                                          ; preds = %block_7, %block_7.preheader.new
  %local_b.06 = phi i64 [ 1, %block_7.preheader.new ], [ %v57.7, %block_7 ]
  %local_a.05 = phi i64 [ 0, %block_7.preheader.new ], [ %v57.6, %block_7 ]
  %niter = phi i64 [ 0, %block_7.preheader.new ], [ %niter.next.7, %block_7 ]
  %v57 = add i64 %local_b.06, %local_a.05
  %v57.1 = add i64 %v57, %local_b.06
  %v57.2 = add i64 %v57.1, %v57
  %v57.3 = add i64 %v57.2, %v57.1
  %v57.4 = add i64 %v57.3, %v57.2
  %v57.5 = add i64 %v57.4, %v57.3
  %v57.6 = add i64 %v57.5, %v57.4
  %v57.7 = add i64 %v57.6, %v57.5
  %niter.next.7 = add i64 %niter, 8
  %niter.ncmp.7 = icmp eq i64 %niter.next.7, %unroll_iter
  br i1 %niter.ncmp.7, label %block_8.loopexit.unr-lcssa, label %block_7

block_8.loopexit.unr-lcssa:                       ; preds = %block_7, %block_7.preheader
  %local_b.06.lcssa.ph = phi i64 [ undef, %block_7.preheader ], [ %v57.6, %block_7 ]
  %local_b.06.unr = phi i64 [ 1, %block_7.preheader ], [ %v57.7, %block_7 ]
  %local_a.05.unr = phi i64 [ 0, %block_7.preheader ], [ %v57.6, %block_7 ]
  %lcmp.mod.not = icmp eq i64 %xtraiter, 0
  br i1 %lcmp.mod.not, label %block_8, label %block_7.epil

block_7.epil:                                     ; preds = %block_8.loopexit.unr-lcssa, %block_7.epil
  %local_b.06.epil = phi i64 [ %v57.epil, %block_7.epil ], [ %local_b.06.unr, %block_8.loopexit.unr-lcssa ]
  %local_a.05.epil = phi i64 [ %local_b.06.epil, %block_7.epil ], [ %local_a.05.unr, %block_8.loopexit.unr-lcssa ]
  %epil.iter = phi i64 [ %epil.iter.next, %block_7.epil ], [ 0, %block_8.loopexit.unr-lcssa ]
  %v57.epil = add i64 %local_b.06.epil, %local_a.05.epil
  %epil.iter.next = add i64 %epil.iter, 1
  %epil.iter.cmp.not = icmp eq i64 %epil.iter.next, %xtraiter
  br i1 %epil.iter.cmp.not, label %block_8, label %block_7.epil, !llvm.loop !0

block_8:                                          ; preds = %block_8.loopexit.unr-lcssa, %block_7.epil, %block_5
  %local_a.0.lcssa = phi i64 [ 0, %block_5 ], [ %local_b.06.lcssa.ph, %block_8.loopexit.unr-lcssa ], [ %local_b.06.epil, %block_7.epil ]
  ret i64 %local_a.0.lcssa
}

; Function Attrs: nofree norecurse nosync nounwind memory(none)
define noundef i64 @agam_count_primes(i64 noundef %p0) local_unnamed_addr #4 {
block_10:
  %v787 = icmp sgt i64 %p0, 2
  br i1 %v787, label %block_12, label %block_13

block_12:                                         ; preds = %block_10, %block_16
  %local_num.09 = phi i64 [ %v107, %block_16 ], [ 2, %block_10 ]
  %local_count.08 = phi i64 [ %v103, %block_16 ], [ 0, %block_10 ]
  %v89.not5 = icmp ult i64 %local_num.09, 4
  br i1 %v89.not5, label %block_16, label %block_15

block_15:                                         ; preds = %block_12, %block_15
  %storemerge6 = phi i64 [ %v99, %block_15 ], [ 2, %block_12 ]
  %0 = phi i64 [ %spec.select, %block_15 ], [ 1, %block_12 ]
  %v92 = srem i64 %local_num.09, %storemerge6
  %v94 = icmp eq i64 %v92, 0
  %spec.select = select i1 %v94, i64 0, i64 %0
  %v99 = add i64 %storemerge6, 1
  %v87 = mul i64 %v99, %v99
  %v89.not = icmp sgt i64 %v87, %local_num.09
  br i1 %v89.not, label %block_16, label %block_15

block_16:                                         ; preds = %block_15, %block_12
  %.lcssa = phi i64 [ 1, %block_12 ], [ %spec.select, %block_15 ]
  %v103 = add i64 %.lcssa, %local_count.08
  %v107 = add nuw nsw i64 %local_num.09, 1
  %exitcond.not = icmp eq i64 %v107, %p0
  br i1 %exitcond.not, label %block_13, label %block_12

block_13:                                         ; preds = %block_16, %block_10
  %local_count.0.lcssa = phi i64 [ 0, %block_10 ], [ %v103, %block_16 ]
  ret i64 %local_count.0.lcssa
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none)
define noundef i64 @agam_matrix_multiply(i64 noundef %p0) local_unnamed_addr #5 {
block_21:
  %v12017 = icmp sgt i64 %p0, 0
  br i1 %v12017, label %block_25.preheader.us.preheader, label %block_24

block_25.preheader.us.preheader:                  ; preds = %block_21
  %0 = add nsw i64 %p0, -1
  %1 = zext nneg i64 %0 to i65
  %2 = add nsw i64 %p0, -2
  %3 = zext i64 %2 to i65
  %4 = mul i65 %1, %3
  %5 = add nsw i64 %p0, -3
  %6 = zext i64 %5 to i65
  %7 = mul i65 %4, %6
  %8 = lshr i65 %7, 1
  %9 = trunc i65 %8 to i64
  %10 = mul i64 %9, 6148914691236517206
  %11 = add i64 %10, %p0
  %12 = lshr i65 %4, 1
  %13 = trunc i65 %12 to i64
  %14 = mul i64 %13, 3
  %15 = add i64 %11, %14
  %16 = add i64 %15, -1
  %17 = mul i64 %16, %p0
  %18 = add i64 %13, %p0
  %19 = mul i64 %15, %p0
  %20 = add i64 %19, %13
  %21 = add i64 %20, -1
  %22 = mul i64 %21, %0
  %23 = mul i64 %18, %0
  %24 = add i64 %23, %p0
  %25 = shl i64 %13, 1
  %26 = add i64 %24, %25
  %27 = add i64 %26, -1
  %28 = add i64 %26, 1
  %29 = mul i64 %28, %p0
  %30 = mul i64 %13, 6
  %31 = add i64 %29, %30
  %32 = add i64 %31, -1
  %.neg = mul i64 %9, -6148914691236517204
  %33 = add i64 %.neg, %32
  %34 = mul i64 %33, %p0
  %35 = add i64 %34, %22
  %36 = mul i64 %18, %13
  %37 = add i64 %35, %36
  %38 = add i64 %37, -1
  %39 = mul i64 %38, %0
  %40 = mul i64 %27, %p0
  %41 = add i64 %40, 1
  %42 = mul i64 %41, %p0
  %43 = add i64 %42, %13
  %44 = add i64 %43, -1
  %45 = mul i64 %44, %13
  %46 = add i64 %17, %39
  %47 = add i64 %46, %45
  br label %block_24

block_24:                                         ; preds = %block_25.preheader.us.preheader, %block_21
  %local_sum.0.lcssa = phi i64 [ 0, %block_21 ], [ %47, %block_25.preheader.us.preheader ]
  ret i64 %local_sum.0.lcssa
}

; Function Attrs: nofree norecurse nosync nounwind memory(none)
define noundef i64 @agam_integrate_x2(i64 noundef %p0) local_unnamed_addr #4 {
block_32:
  %v1847 = icmp sgt i64 %p0, 0
  br i1 %v1847, label %block_34.preheader, label %block_35

block_34.preheader:                               ; preds = %block_32
  %v17511 = urem i64 %p0, 1237
  %v177 = add nuw nsw i64 %v17511, 3
  br label %block_34

block_34:                                         ; preds = %block_34.preheader, %block_34
  %local_i.010 = phi i64 [ %v205, %block_34 ], [ 0, %block_34.preheader ]
  %local_wobble.09 = phi i64 [ %v191, %block_34 ], [ %v177, %block_34.preheader ]
  %local_sum.08 = phi i64 [ %v201, %block_34 ], [ 0, %block_34.preheader ]
  %v187 = mul nsw i64 %local_wobble.09, 73
  %v189 = add nsw i64 %v187, 19
  %v191 = srem i64 %v189, 65521
  %v196 = mul i64 %local_i.010, %local_i.010
  %v198 = add i64 %v191, %v196
  %v200 = srem i64 %v198, 4096
  %v201 = add i64 %v200, %local_sum.08
  %v205 = add nuw nsw i64 %local_i.010, 1
  %exitcond.not = icmp eq i64 %v205, %p0
  br i1 %exitcond.not, label %block_35, label %block_34

block_35:                                         ; preds = %block_34, %block_32
  %local_sum.0.lcssa = phi i64 [ 0, %block_32 ], [ %v201, %block_34 ]
  ret i64 %local_sum.0.lcssa
}

; Function Attrs: mustprogress nofree norecurse nounwind willreturn
define noundef i64 @agam_arg_or(i32 noundef %p0, i64 noundef %p1) local_unnamed_addr #6 {
block_37:
  %0 = load i32, ptr @agam_argc_storage, align 4
  %v213 = icmp sgt i32 %0, %p0
  br i1 %v213, label %block_38, label %common.ret

common.ret:                                       ; preds = %block_37, %block_38
  %common.ret.op = phi i64 [ %tmp39, %block_38 ], [ %p1, %block_37 ]
  ret i64 %common.ret.op

block_38:                                         ; preds = %block_37
  %1 = load ptr, ptr @agam_argv_storage, align 8
  %2 = sext i32 %p0 to i64
  %3 = getelementptr inbounds ptr, ptr %1, i64 %2
  %4 = load ptr, ptr %3, align 8
  %5 = tail call i64 @strtoll(ptr nocapture noundef %4, ptr null, i32 noundef 10)
  %sext = shl i64 %5, 32
  %tmp39 = ashr exact i64 %sext, 32
  br label %common.ret
}

; Function Attrs: nofree nounwind
define noundef i32 @main(i32 noundef %argc, ptr noundef %argv) local_unnamed_addr #0 {
block_43:
  store i32 %argc, ptr @agam_argc_storage, align 4
  store ptr %argv, ptr @agam_argv_storage, align 8
  %v213.i = icmp sgt i32 %argc, 1
  br i1 %v213.i, label %agam_arg_or.exit, label %agam_arg_or.exit33

agam_arg_or.exit:                                 ; preds = %block_43
  %0 = getelementptr inbounds ptr, ptr %argv, i64 1
  %1 = load ptr, ptr %0, align 8
  %2 = tail call i64 @strtoll(ptr nocapture noundef %1, ptr null, i32 noundef 10)
  %sext.i = shl i64 %2, 32
  %tmp39.i = ashr exact i64 %sext.i, 32
  %.pr = load i32, ptr @agam_argc_storage, align 4
  %v213.i10 = icmp sgt i32 %.pr, 2
  br i1 %v213.i10, label %agam_arg_or.exit15, label %agam_arg_or.exit33

agam_arg_or.exit15:                               ; preds = %agam_arg_or.exit
  %3 = load ptr, ptr @agam_argv_storage, align 8
  %4 = getelementptr inbounds ptr, ptr %3, i64 2
  %5 = load ptr, ptr %4, align 8
  %6 = tail call i64 @strtoll(ptr nocapture noundef %5, ptr null, i32 noundef 10)
  %sext.i13 = shl i64 %6, 32
  %tmp39.i14 = ashr exact i64 %sext.i13, 32
  %.pr42 = load i32, ptr @agam_argc_storage, align 4
  %v213.i16 = icmp sgt i32 %.pr42, 3
  br i1 %v213.i16, label %agam_arg_or.exit21, label %agam_arg_or.exit33

agam_arg_or.exit21:                               ; preds = %agam_arg_or.exit15
  %7 = load ptr, ptr @agam_argv_storage, align 8
  %8 = getelementptr inbounds ptr, ptr %7, i64 3
  %9 = load ptr, ptr %8, align 8
  %10 = tail call i64 @strtoll(ptr nocapture noundef %9, ptr null, i32 noundef 10)
  %sext.i19 = shl i64 %10, 32
  %tmp39.i20 = ashr exact i64 %sext.i19, 32
  %.pr48.pr = load i32, ptr @agam_argc_storage, align 4
  %v213.i22 = icmp sgt i32 %.pr48.pr, 4
  br i1 %v213.i22, label %agam_arg_or.exit27, label %agam_arg_or.exit33

agam_arg_or.exit27:                               ; preds = %agam_arg_or.exit21
  %11 = load ptr, ptr @agam_argv_storage, align 8
  %12 = getelementptr inbounds ptr, ptr %11, i64 4
  %13 = load ptr, ptr %12, align 8
  %14 = tail call i64 @strtoll(ptr nocapture noundef %13, ptr null, i32 noundef 10)
  %sext.i25 = shl i64 %14, 32
  %tmp39.i26 = ashr exact i64 %sext.i25, 32
  %.pr56 = load i32, ptr @agam_argc_storage, align 4
  %v213.i28 = icmp sgt i32 %.pr56, 5
  br i1 %v213.i28, label %block_38.i30, label %agam_arg_or.exit33

block_38.i30:                                     ; preds = %agam_arg_or.exit27
  %15 = load ptr, ptr @agam_argv_storage, align 8
  %16 = getelementptr inbounds ptr, ptr %15, i64 5
  %17 = load ptr, ptr %16, align 8
  %18 = tail call i64 @strtoll(ptr nocapture noundef %17, ptr null, i32 noundef 10)
  %sext.i31 = shl i64 %18, 32
  %tmp39.i32 = ashr exact i64 %sext.i31, 32
  br label %agam_arg_or.exit33

agam_arg_or.exit33:                               ; preds = %agam_arg_or.exit, %block_43, %agam_arg_or.exit15, %agam_arg_or.exit21, %agam_arg_or.exit27, %block_38.i30
  %common.ret.op.i2365 = phi i64 [ %tmp39.i26, %block_38.i30 ], [ %tmp39.i26, %agam_arg_or.exit27 ], [ 100, %agam_arg_or.exit21 ], [ 100, %agam_arg_or.exit15 ], [ 100, %block_43 ], [ 100, %agam_arg_or.exit ]
  %common.ret.op.i11475364 = phi i64 [ %tmp39.i14, %block_38.i30 ], [ %tmp39.i14, %agam_arg_or.exit27 ], [ %tmp39.i14, %agam_arg_or.exit21 ], [ %tmp39.i14, %agam_arg_or.exit15 ], [ 40, %block_43 ], [ 40, %agam_arg_or.exit ]
  %common.ret.op.i41465463 = phi i64 [ %tmp39.i, %block_38.i30 ], [ %tmp39.i, %agam_arg_or.exit27 ], [ %tmp39.i, %agam_arg_or.exit21 ], [ %tmp39.i, %agam_arg_or.exit15 ], [ 100000000, %block_43 ], [ %tmp39.i, %agam_arg_or.exit ]
  %common.ret.op.i175562 = phi i64 [ %tmp39.i20, %block_38.i30 ], [ %tmp39.i20, %agam_arg_or.exit27 ], [ %tmp39.i20, %agam_arg_or.exit21 ], [ 100000, %agam_arg_or.exit15 ], [ 100000, %block_43 ], [ 100000, %agam_arg_or.exit ]
  %common.ret.op.i29 = phi i64 [ %tmp39.i32, %block_38.i30 ], [ 10000000, %agam_arg_or.exit27 ], [ 10000000, %agam_arg_or.exit21 ], [ 10000000, %agam_arg_or.exit15 ], [ 10000000, %block_43 ], [ 10000000, %agam_arg_or.exit ]
  %puts = tail call i32 @puts(ptr nonnull dereferenceable(1) @.str.0)
  %tmp46 = tail call i32 (ptr, ...) @printf(ptr nonnull dereferenceable(1) @.str.2, i64 %common.ret.op.i41465463)
  %v167.i = icmp sgt i64 %common.ret.op.i41465463, 0
  br i1 %v167.i, label %block_2.preheader.i, label %agam_sum_loop.exit

block_2.preheader.i:                              ; preds = %agam_arg_or.exit33
  %v711.i.lhs.trunc = trunc i64 %common.ret.op.i41465463 to i32
  %v711.i72 = urem i32 %v711.i.lhs.trunc, 7919
  %narrow = add nuw nsw i32 %v711.i72, 1
  %v9.i = zext nneg i32 %narrow to i64
  br label %block_2.i

block_2.i:                                        ; preds = %block_2.i, %block_2.preheader.i
  %local_i.010.i = phi i64 [ %v37.i, %block_2.i ], [ 0, %block_2.preheader.i ]
  %local_state.09.i = phi i64 [ %v27.i, %block_2.i ], [ %v9.i, %block_2.preheader.i ]
  %local_total.08.i = phi i64 [ %v33.i, %block_2.i ], [ 0, %block_2.preheader.i ]
  %v19.i = mul nsw i64 %local_state.09.i, 57
  %v22.i = mul nuw nsw i64 %local_i.010.i, 13
  %v23.i = add nuw nsw i64 %v22.i, 17
  %v25.i = add i64 %v23.i, %v19.i
  %v27.i = srem i64 %v25.i, 1000003
  %v32.lhs.trunc.i = trunc i64 %v27.i to i32
  %v326.i = srem i32 %v32.lhs.trunc.i, 1024
  %v32.sext.i = sext i32 %v326.i to i64
  %v33.i = add i64 %local_total.08.i, %v32.sext.i
  %v37.i = add nuw nsw i64 %local_i.010.i, 1
  %exitcond.not.i = icmp eq i64 %v37.i, %common.ret.op.i41465463
  br i1 %exitcond.not.i, label %agam_sum_loop.exit, label %block_2.i

agam_sum_loop.exit:                               ; preds = %block_2.i, %agam_arg_or.exit33
  %local_total.0.lcssa.i = phi i64 [ 0, %agam_arg_or.exit33 ], [ %v33.i, %block_2.i ]
  %tmp47 = tail call i32 (ptr, ...) @printf(ptr nonnull dereferenceable(1) @.str.2, i64 %local_total.0.lcssa.i)
  %puts6 = tail call i32 @puts(ptr nonnull dereferenceable(1) @.str.3)
  %tmp49 = tail call i32 (ptr, ...) @printf(ptr nonnull dereferenceable(1) @.str.2, i64 %common.ret.op.i11475364)
  %v534.i = icmp sgt i64 %common.ret.op.i11475364, 0
  br i1 %v534.i, label %block_7.i.preheader, label %agam_fibonacci.exit

block_7.i.preheader:                              ; preds = %agam_sum_loop.exit
  %xtraiter = and i64 %common.ret.op.i11475364, 7
  %19 = icmp ult i64 %common.ret.op.i11475364, 8
  br i1 %19, label %agam_fibonacci.exit.loopexit.unr-lcssa, label %block_7.i.preheader.new

block_7.i.preheader.new:                          ; preds = %block_7.i.preheader
  %unroll_iter = and i64 %common.ret.op.i11475364, 9223372036854775800
  br label %block_7.i

block_7.i:                                        ; preds = %block_7.i, %block_7.i.preheader.new
  %local_b.06.i = phi i64 [ 1, %block_7.i.preheader.new ], [ %v57.i.7, %block_7.i ]
  %local_a.05.i = phi i64 [ 0, %block_7.i.preheader.new ], [ %v57.i.6, %block_7.i ]
  %niter = phi i64 [ 0, %block_7.i.preheader.new ], [ %niter.next.7, %block_7.i ]
  %v57.i = add i64 %local_a.05.i, %local_b.06.i
  %v57.i.1 = add i64 %local_b.06.i, %v57.i
  %v57.i.2 = add i64 %v57.i, %v57.i.1
  %v57.i.3 = add i64 %v57.i.1, %v57.i.2
  %v57.i.4 = add i64 %v57.i.2, %v57.i.3
  %v57.i.5 = add i64 %v57.i.3, %v57.i.4
  %v57.i.6 = add i64 %v57.i.4, %v57.i.5
  %v57.i.7 = add i64 %v57.i.5, %v57.i.6
  %niter.next.7 = add i64 %niter, 8
  %niter.ncmp.7 = icmp eq i64 %niter.next.7, %unroll_iter
  br i1 %niter.ncmp.7, label %agam_fibonacci.exit.loopexit.unr-lcssa, label %block_7.i

agam_fibonacci.exit.loopexit.unr-lcssa:           ; preds = %block_7.i, %block_7.i.preheader
  %local_b.06.i.lcssa.ph = phi i64 [ undef, %block_7.i.preheader ], [ %v57.i.6, %block_7.i ]
  %local_b.06.i.unr = phi i64 [ 1, %block_7.i.preheader ], [ %v57.i.7, %block_7.i ]
  %local_a.05.i.unr = phi i64 [ 0, %block_7.i.preheader ], [ %v57.i.6, %block_7.i ]
  %lcmp.mod.not = icmp eq i64 %xtraiter, 0
  br i1 %lcmp.mod.not, label %agam_fibonacci.exit, label %block_7.i.epil

block_7.i.epil:                                   ; preds = %agam_fibonacci.exit.loopexit.unr-lcssa, %block_7.i.epil
  %local_b.06.i.epil = phi i64 [ %v57.i.epil, %block_7.i.epil ], [ %local_b.06.i.unr, %agam_fibonacci.exit.loopexit.unr-lcssa ]
  %local_a.05.i.epil = phi i64 [ %local_b.06.i.epil, %block_7.i.epil ], [ %local_a.05.i.unr, %agam_fibonacci.exit.loopexit.unr-lcssa ]
  %epil.iter = phi i64 [ %epil.iter.next, %block_7.i.epil ], [ 0, %agam_fibonacci.exit.loopexit.unr-lcssa ]
  %v57.i.epil = add i64 %local_a.05.i.epil, %local_b.06.i.epil
  %epil.iter.next = add i64 %epil.iter, 1
  %epil.iter.cmp.not = icmp eq i64 %epil.iter.next, %xtraiter
  br i1 %epil.iter.cmp.not, label %agam_fibonacci.exit, label %block_7.i.epil, !llvm.loop !2

agam_fibonacci.exit:                              ; preds = %agam_fibonacci.exit.loopexit.unr-lcssa, %block_7.i.epil, %agam_sum_loop.exit
  %local_a.0.lcssa.i = phi i64 [ 0, %agam_sum_loop.exit ], [ %local_b.06.i.lcssa.ph, %agam_fibonacci.exit.loopexit.unr-lcssa ], [ %local_b.06.i.epil, %block_7.i.epil ]
  %tmp50 = tail call i32 (ptr, ...) @printf(ptr nonnull dereferenceable(1) @.str.2, i64 %local_a.0.lcssa.i)
  %puts7 = tail call i32 @puts(ptr nonnull dereferenceable(1) @.str.4)
  %tmp52 = tail call i32 (ptr, ...) @printf(ptr nonnull dereferenceable(1) @.str.2, i64 %common.ret.op.i175562)
  %v787.i = icmp sgt i64 %common.ret.op.i175562, 2
  br i1 %v787.i, label %block_12.i, label %agam_count_primes.exit

block_12.i:                                       ; preds = %agam_fibonacci.exit, %block_16.i
  %local_num.09.i = phi i64 [ %v107.i, %block_16.i ], [ 2, %agam_fibonacci.exit ]
  %local_count.08.i = phi i64 [ %v103.i, %block_16.i ], [ 0, %agam_fibonacci.exit ]
  %v89.not5.i = icmp ult i64 %local_num.09.i, 4
  br i1 %v89.not5.i, label %block_16.i, label %block_15.i

block_15.i:                                       ; preds = %block_12.i, %block_15.i
  %storemerge6.i = phi i64 [ %v99.i, %block_15.i ], [ 2, %block_12.i ]
  %20 = phi i64 [ %spec.select.i, %block_15.i ], [ 1, %block_12.i ]
  %v92.i = srem i64 %local_num.09.i, %storemerge6.i
  %v94.i = icmp eq i64 %v92.i, 0
  %spec.select.i = select i1 %v94.i, i64 0, i64 %20
  %v99.i = add i64 %storemerge6.i, 1
  %v87.i = mul i64 %v99.i, %v99.i
  %v89.not.i = icmp sgt i64 %v87.i, %local_num.09.i
  br i1 %v89.not.i, label %block_16.i, label %block_15.i

block_16.i:                                       ; preds = %block_15.i, %block_12.i
  %.lcssa.i = phi i64 [ 1, %block_12.i ], [ %spec.select.i, %block_15.i ]
  %v103.i = add i64 %.lcssa.i, %local_count.08.i
  %v107.i = add nuw nsw i64 %local_num.09.i, 1
  %exitcond.not.i35 = icmp eq i64 %v107.i, %common.ret.op.i175562
  br i1 %exitcond.not.i35, label %agam_count_primes.exit, label %block_12.i

agam_count_primes.exit:                           ; preds = %block_16.i, %agam_fibonacci.exit
  %local_count.0.lcssa.i = phi i64 [ 0, %agam_fibonacci.exit ], [ %v103.i, %block_16.i ]
  %tmp53 = tail call i32 (ptr, ...) @printf(ptr nonnull dereferenceable(1) @.str.2, i64 %local_count.0.lcssa.i)
  %puts8 = tail call i32 @puts(ptr nonnull dereferenceable(1) @.str.5)
  %tmp55 = tail call i32 (ptr, ...) @printf(ptr nonnull dereferenceable(1) @.str.2, i64 %common.ret.op.i2365)
  %v12017.i = icmp sgt i64 %common.ret.op.i2365, 0
  br i1 %v12017.i, label %block_25.preheader.us.preheader.i, label %agam_matrix_multiply.exit

block_25.preheader.us.preheader.i:                ; preds = %agam_count_primes.exit
  %21 = add nsw i64 %common.ret.op.i2365, -1
  %22 = zext nneg i64 %21 to i65
  %23 = add nsw i64 %common.ret.op.i2365, -2
  %24 = zext i64 %23 to i65
  %25 = mul i65 %22, %24
  %26 = add nsw i64 %common.ret.op.i2365, -3
  %27 = zext i64 %26 to i65
  %28 = mul i65 %25, %27
  %29 = lshr i65 %28, 1
  %30 = trunc i65 %29 to i64
  %31 = mul i64 %30, 6148914691236517206
  %32 = lshr i65 %25, 1
  %33 = trunc i65 %32 to i64
  %34 = mul i64 %33, 3
  %35 = add i64 %34, %common.ret.op.i2365
  %36 = add i64 %35, %31
  %37 = add i64 %36, -1
  %38 = mul i64 %37, %common.ret.op.i2365
  %39 = add i64 %common.ret.op.i2365, %33
  %40 = mul i64 %36, %common.ret.op.i2365
  %41 = add i64 %33, -1
  %42 = add i64 %41, %40
  %43 = mul i64 %42, %21
  %44 = mul i64 %39, %21
  %45 = shl i64 %33, 1
  %46 = add i64 %45, %common.ret.op.i2365
  %47 = add i64 %46, %44
  %48 = add i64 %47, -1
  %49 = add i64 %47, 1
  %50 = mul i64 %49, %common.ret.op.i2365
  %51 = mul i64 %33, 6
  %.neg.i = mul i64 %30, -6148914691236517204
  %52 = add i64 %51, -1
  %53 = add i64 %52, %.neg.i
  %54 = add i64 %53, %50
  %55 = mul i64 %54, %common.ret.op.i2365
  %56 = mul i64 %39, %33
  %57 = add i64 %56, -1
  %58 = add i64 %57, %43
  %59 = add i64 %58, %55
  %60 = mul i64 %59, %21
  %61 = mul i64 %48, %common.ret.op.i2365
  %62 = add i64 %61, 1
  %63 = mul i64 %62, %common.ret.op.i2365
  %64 = add i64 %41, %63
  %65 = mul i64 %64, %33
  %66 = add i64 %65, %38
  %67 = add i64 %66, %60
  br label %agam_matrix_multiply.exit

agam_matrix_multiply.exit:                        ; preds = %agam_count_primes.exit, %block_25.preheader.us.preheader.i
  %local_sum.0.lcssa.i = phi i64 [ 0, %agam_count_primes.exit ], [ %67, %block_25.preheader.us.preheader.i ]
  %tmp56 = tail call i32 (ptr, ...) @printf(ptr nonnull dereferenceable(1) @.str.2, i64 %local_sum.0.lcssa.i)
  %puts9 = tail call i32 @puts(ptr nonnull dereferenceable(1) @.str.6)
  %tmp58 = tail call i32 (ptr, ...) @printf(ptr nonnull dereferenceable(1) @.str.2, i64 %common.ret.op.i29)
  %v1847.i = icmp sgt i64 %common.ret.op.i29, 0
  br i1 %v1847.i, label %block_34.preheader.i, label %agam_integrate_x2.exit

block_34.preheader.i:                             ; preds = %agam_matrix_multiply.exit
  %v17511.i = urem i64 %common.ret.op.i29, 1237
  %v177.i = add nuw nsw i64 %v17511.i, 3
  br label %block_34.i

block_34.i:                                       ; preds = %block_34.i, %block_34.preheader.i
  %local_i.010.i37 = phi i64 [ %v205.i, %block_34.i ], [ 0, %block_34.preheader.i ]
  %local_wobble.09.i = phi i64 [ %v191.i, %block_34.i ], [ %v177.i, %block_34.preheader.i ]
  %local_sum.08.i = phi i64 [ %v201.i, %block_34.i ], [ 0, %block_34.preheader.i ]
  %v187.i = mul nuw nsw i64 %local_wobble.09.i, 73
  %v189.i = add nuw nsw i64 %v187.i, 19
  %v191.i = urem i64 %v189.i, 65521
  %v196.i = mul i64 %local_i.010.i37, %local_i.010.i37
  %v198.i = add i64 %v191.i, %v196.i
  %v200.i = srem i64 %v198.i, 4096
  %v201.i = add i64 %v200.i, %local_sum.08.i
  %v205.i = add nuw nsw i64 %local_i.010.i37, 1
  %exitcond.not.i38 = icmp eq i64 %v205.i, %common.ret.op.i29
  br i1 %exitcond.not.i38, label %agam_integrate_x2.exit, label %block_34.i

agam_integrate_x2.exit:                           ; preds = %block_34.i, %agam_matrix_multiply.exit
  %local_sum.0.lcssa.i36 = phi i64 [ 0, %agam_matrix_multiply.exit ], [ %v201.i, %block_34.i ]
  %tmp59 = tail call i32 (ptr, ...) @printf(ptr nonnull dereferenceable(1) @.str.2, i64 %local_sum.0.lcssa.i36)
  ret i32 0
}

; Function Attrs: nofree nounwind
declare noundef i32 @puts(ptr nocapture noundef readonly) local_unnamed_addr #0

attributes #0 = { nofree nounwind }
attributes #1 = { mustprogress nofree nounwind willreturn }
attributes #2 = { mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: none, inaccessiblemem: none) }
attributes #3 = { mustprogress nofree norecurse nosync nounwind willreturn memory(read, inaccessiblemem: none) }
attributes #4 = { nofree norecurse nosync nounwind memory(none) }
attributes #5 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) }
attributes #6 = { mustprogress nofree norecurse nounwind willreturn }

!0 = distinct !{!0, !1}
!1 = !{!"llvm.loop.unroll.disable"}
!2 = distinct !{!2, !1}
