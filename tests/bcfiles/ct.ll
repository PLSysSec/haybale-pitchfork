; ModuleID = 'ct.c'
source_filename = "ct.c"
target datalayout = "e-m:o-i64:64-f80:128-n8:16:32:64-S128"
target triple = "x86_64-apple-macosx10.14.0"

%struct.PartiallySecret = type { i32, i32 }

@__const.notct_onepath.z = private unnamed_addr constant [3 x i32] [i32 0, i32 2, i32 300], align 4

; Function Attrs: norecurse nounwind readnone ssp uwtable
define i32 @ct_simple(i32) local_unnamed_addr #0 {
  %2 = add nsw i32 %0, 3
  ret i32 %2
}

; Function Attrs: nounwind ssp uwtable
define i32 @ct_simple2(i32, i32) local_unnamed_addr #1 {
  %3 = alloca i32, align 4
  %4 = bitcast i32* %3 to i8*
  call void @llvm.lifetime.start.p0i8(i64 4, i8* nonnull %4)
  store volatile i32 2, i32* %3, align 4, !tbaa !3
  %5 = load volatile i32, i32* %3, align 4, !tbaa !3
  %6 = icmp sgt i32 %5, 3
  br i1 %6, label %7, label %9

; <label>:7:                                      ; preds = %2
  %8 = mul nsw i32 %0, 5
  br label %11

; <label>:9:                                      ; preds = %2
  %10 = sdiv i32 %1, 99
  br label %11

; <label>:11:                                     ; preds = %9, %7
  %12 = phi i32 [ %8, %7 ], [ %10, %9 ]
  call void @llvm.lifetime.end.p0i8(i64 4, i8* nonnull %4)
  ret i32 %12
}

; Function Attrs: argmemonly nounwind
declare void @llvm.lifetime.start.p0i8(i64, i8* nocapture) #2

; Function Attrs: argmemonly nounwind
declare void @llvm.lifetime.end.p0i8(i64, i8* nocapture) #2

; Function Attrs: norecurse nounwind readnone ssp uwtable
define i32 @notct_branch(i32) local_unnamed_addr #0 {
  %2 = icmp sgt i32 %0, 10
  br i1 %2, label %3, label %6

; <label>:3:                                      ; preds = %1
  %4 = urem i32 %0, 200
  %5 = mul nuw nsw i32 %4, 3
  br label %8

; <label>:6:                                      ; preds = %1
  %7 = add nsw i32 %0, 10
  br label %8

; <label>:8:                                      ; preds = %6, %3
  %9 = phi i32 [ %5, %3 ], [ %7, %6 ]
  ret i32 %9
}

; Function Attrs: nounwind ssp uwtable
define i32 @notct_mem(i32) local_unnamed_addr #1 {
  %2 = alloca [3 x i32], align 4
  %3 = bitcast [3 x i32]* %2 to i8*
  call void @llvm.lifetime.start.p0i8(i64 12, i8* nonnull %3) #4
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* nonnull align 4 %3, i8* align 4 bitcast ([3 x i32]* @__const.notct_onepath.z to i8*), i64 12, i1 true)
  %4 = srem i32 %0, 3
  %5 = sext i32 %4 to i64
  %6 = getelementptr inbounds [3 x i32], [3 x i32]* %2, i64 0, i64 %5
  %7 = load volatile i32, i32* %6, align 4, !tbaa !3
  call void @llvm.lifetime.end.p0i8(i64 12, i8* nonnull %3) #4
  ret i32 %7
}

; Function Attrs: argmemonly nounwind
declare void @llvm.memcpy.p0i8.p0i8.i64(i8* nocapture writeonly, i8* nocapture readonly, i64, i1) #2

; Function Attrs: nounwind ssp uwtable
define i32 @notct_onepath(i32, i32) local_unnamed_addr #1 {
  %3 = alloca [3 x i32], align 4
  %4 = bitcast [3 x i32]* %3 to i8*
  call void @llvm.lifetime.start.p0i8(i64 12, i8* nonnull %4) #4
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* nonnull align 4 %4, i8* align 4 bitcast ([3 x i32]* @__const.notct_onepath.z to i8*), i64 12, i1 true)
  %5 = getelementptr inbounds [3 x i32], [3 x i32]* %3, i64 0, i64 2
  store volatile i32 %1, i32* %5, align 4, !tbaa !3
  %6 = load volatile i32, i32* %5, align 4, !tbaa !3
  %7 = icmp sgt i32 %6, 3
  br i1 %7, label %8, label %11

; <label>:8:                                      ; preds = %2
  %9 = srem i32 %0, 3
  %10 = sext i32 %9 to i64
  br label %11

; <label>:11:                                     ; preds = %2, %8
  %12 = phi i64 [ %10, %8 ], [ 1, %2 ]
  %13 = getelementptr inbounds [3 x i32], [3 x i32]* %3, i64 0, i64 %12
  %14 = load volatile i32, i32* %13, align 4, !tbaa !3
  call void @llvm.lifetime.end.p0i8(i64 12, i8* nonnull %4) #4
  ret i32 %14
}

; Function Attrs: norecurse nounwind readnone ssp uwtable
define i32 @ct_onearg(i32, i32) local_unnamed_addr #0 {
  %3 = icmp sgt i32 %0, 100
  br i1 %3, label %7, label %4

; <label>:4:                                      ; preds = %2
  %5 = srem i32 %0, 20
  %6 = mul nsw i32 %5, 3
  br label %7

; <label>:7:                                      ; preds = %2, %4
  %8 = phi i32 [ %6, %4 ], [ %1, %2 ]
  ret i32 %8
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @ct_secrets(i32* nocapture readonly) local_unnamed_addr #3 {
  %2 = getelementptr inbounds i32, i32* %0, i64 20
  %3 = load i32, i32* %2, align 4, !tbaa !3
  %4 = add nsw i32 %3, 3
  ret i32 %4
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @notct_secrets(i32* nocapture readonly) local_unnamed_addr #3 {
  %2 = getelementptr inbounds i32, i32* %0, i64 20
  %3 = load i32, i32* %2, align 4, !tbaa !3
  %4 = icmp sgt i32 %3, 3
  br i1 %4, label %5, label %8

; <label>:5:                                      ; preds = %1
  %6 = load i32, i32* %0, align 4, !tbaa !3
  %7 = mul nsw i32 %6, 3
  br label %12

; <label>:8:                                      ; preds = %1
  %9 = getelementptr inbounds i32, i32* %0, i64 2
  %10 = load i32, i32* %9, align 4, !tbaa !3
  %11 = sdiv i32 %10, 22
  br label %12

; <label>:12:                                     ; preds = %8, %5
  %13 = phi i32 [ %7, %5 ], [ %11, %8 ]
  ret i32 %13
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @ct_struct(i32* nocapture readonly, %struct.PartiallySecret* nocapture readonly) local_unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.PartiallySecret, %struct.PartiallySecret* %1, i64 0, i32 0
  %4 = load i32, i32* %3, align 4, !tbaa !7
  %5 = sext i32 %4 to i64
  %6 = getelementptr inbounds i32, i32* %0, i64 %5
  %7 = load i32, i32* %6, align 4, !tbaa !3
  %8 = getelementptr inbounds %struct.PartiallySecret, %struct.PartiallySecret* %1, i64 0, i32 1
  %9 = load i32, i32* %8, align 4, !tbaa !9
  %10 = add nsw i32 %9, %7
  ret i32 %10
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @notct_struct(i32* nocapture readonly, %struct.PartiallySecret* nocapture readonly) local_unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.PartiallySecret, %struct.PartiallySecret* %1, i64 0, i32 1
  %4 = load i32, i32* %3, align 4, !tbaa !9
  %5 = sext i32 %4 to i64
  %6 = getelementptr inbounds i32, i32* %0, i64 %5
  %7 = load i32, i32* %6, align 4, !tbaa !3
  %8 = getelementptr inbounds %struct.PartiallySecret, %struct.PartiallySecret* %1, i64 0, i32 0
  %9 = load i32, i32* %8, align 4, !tbaa !7
  %10 = add nsw i32 %9, %7
  ret i32 %10
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @ct_doubleptr(i32** nocapture readonly) local_unnamed_addr #3 {
  %2 = getelementptr inbounds i32*, i32** %0, i64 2
  %3 = load i32*, i32** %2, align 8, !tbaa !10
  %4 = getelementptr inbounds i32, i32* %3, i64 5
  %5 = load i32, i32* %4, align 4, !tbaa !3
  %6 = add nsw i32 %5, 3
  ret i32 %6
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @notct_doubleptr(i32** nocapture readonly) local_unnamed_addr #3 {
  %2 = getelementptr inbounds i32*, i32** %0, i64 2
  %3 = load i32*, i32** %2, align 8, !tbaa !10
  %4 = getelementptr inbounds i32, i32* %3, i64 5
  %5 = load i32, i32* %4, align 4, !tbaa !3
  %6 = icmp sgt i32 %5, 3
  br i1 %6, label %7, label %12

; <label>:7:                                      ; preds = %1
  %8 = load i32*, i32** %0, align 8, !tbaa !10
  %9 = getelementptr inbounds i32, i32* %8, i64 10
  %10 = load i32, i32* %9, align 4, !tbaa !3
  %11 = mul nsw i32 %10, 3
  br label %16

; <label>:12:                                     ; preds = %1
  %13 = getelementptr inbounds i32, i32* %3, i64 22
  %14 = load i32, i32* %13, align 4, !tbaa !3
  %15 = sdiv i32 %14, 5
  br label %16

; <label>:16:                                     ; preds = %12, %7
  %17 = phi i32 [ %11, %7 ], [ %15, %12 ]
  ret i32 %17
}

attributes #0 = { norecurse nounwind readnone ssp uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="penryn" "target-features"="+cx16,+fxsr,+mmx,+sahf,+sse,+sse2,+sse3,+sse4.1,+ssse3,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #1 = { nounwind ssp uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="penryn" "target-features"="+cx16,+fxsr,+mmx,+sahf,+sse,+sse2,+sse3,+sse4.1,+ssse3,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #2 = { argmemonly nounwind }
attributes #3 = { norecurse nounwind readonly ssp uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="penryn" "target-features"="+cx16,+fxsr,+mmx,+sahf,+sse,+sse2,+sse3,+sse4.1,+ssse3,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #4 = { nounwind }

!llvm.module.flags = !{!0, !1}
!llvm.ident = !{!2}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 7, !"PIC Level", i32 2}
!2 = !{!"clang version 8.0.0 (tags/RELEASE_800/final)"}
!3 = !{!4, !4, i64 0}
!4 = !{!"int", !5, i64 0}
!5 = !{!"omnipotent char", !6, i64 0}
!6 = !{!"Simple C/C++ TBAA"}
!7 = !{!8, !4, i64 0}
!8 = !{!"PartiallySecret", !4, i64 0, !4, i64 4}
!9 = !{!8, !4, i64 4}
!10 = !{!11, !11, i64 0}
!11 = !{!"any pointer", !5, i64 0}
